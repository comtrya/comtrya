//! AUR helpers (yay and paru). They build packages as the user and call sudo themselves, and
//! only differ in their name and install arguments

use super::{pacman, PackageProvider};
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::runner::{self, Runner};
use crate::steps::Step;
use crate::utilities;
use anyhow::anyhow;
use std::sync::Arc;
use tracing::warn;
use which::which;

/// What differs between the AUR helpers
struct Helper {
    /// The command, also the name of its AUR package
    command: &'static str,

    /// The name shown in the output
    name: &'static str,

    /// The arguments of the install command, before `extra_args` and the packages
    install_args: &'static [&'static str],
}

/// yay installs the packages without asking
const YAY: Helper = Helper {
    command: "yay",
    name: "Yay",
    install_args: &["-S", "--noconfirm"],
};

/// paru installs the packages without asking, and keeps sudo alive while it builds
const PARU: Helper = Helper {
    command: "paru",
    name: "Paru",
    install_args: &[
        "-Sq",
        "--batchinstall",
        "--needed",
        "--noconfirm",
        "--noprogressbar",
        "--skipreview",
        "--sudoloop",
        "--useask",
    ],
};

/// Packages from the official repositories and the AUR, installed with yay or paru
#[derive(Clone)]
pub struct AurHelper {
    /// Which helper this is
    helper: &'static Helper,
    /// Runs `pacman -Q` to find the installed packages
    runner: Arc<dyn Runner>,
}

impl AurHelper {
    /// yay, built from the AUR if it isn't installed
    pub fn yay() -> Self {
        Self::new(&YAY)
    }

    /// paru, built from the AUR if it isn't installed
    pub fn paru() -> Self {
        Self::new(&PARU)
    }

    /// The helper, with the commands running on this system
    fn new(helper: &'static Helper) -> Self {
        Self {
            helper,
            runner: runner::system(),
        }
    }

    /// Builds the helper from its AUR sources, after installing what `makepkg` needs
    fn build_from_aur(&self, contexts: &Contexts) -> Vec<Step> {
        let command = self.helper.command;
        let build_dir = format!("/tmp/{command}");

        vec![
            Step::new(Exec {
                command: String::from("pacman"),
                arguments: strings(&["-S", "--needed", "--noconfirm", "base-devel", "git"]),
                privileged: true,
                privilege_provider: utilities::get_privilege_provider(contexts),
                ..Default::default()
            }),
            Step::new(Exec {
                command: String::from("git"),
                arguments: vec![
                    String::from("clone"),
                    format!("https://aur.archlinux.org/{command}.git"),
                    build_dir.clone(),
                ],
                ..Default::default()
            }),
            Step::new(Exec {
                command: String::from("makepkg"),
                arguments: strings(&["-si", "--noconfirm"]),
                working_dir: Some(build_dir),
                ..Default::default()
            }),
        ]
    }
}

impl PackageProvider for AurHelper {
    fn name(&self) -> &str {
        self.helper.name
    }

    fn available(&self) -> bool {
        match which(self.helper.command) {
            Ok(_) => true,
            Err(_) => {
                warn!("{} not available", self.helper.command);
                false
            }
        }
    }

    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step> {
        self.build_from_aur(contexts)
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(
        &self,
        _: &PackageRepository,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    /// pacman knows the AUR packages too, once they are installed
    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        pacman::missing(self.runner.as_ref(), package)
    }

    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        refuse_root(self.helper.command, contexts)?;

        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![Step::new(Exec {
            command: String::from(self.helper.command),
            arguments: [
                strings(self.helper.install_args),
                package.extra_args.clone(),
                need_installed,
            ]
            .concat(),
            ..Default::default()
        })])
    }
}

/// AUR helpers (`makepkg`) refuse to run as root, with an error that doesn't say what to do
fn refuse_root(provider: &str, contexts: &Contexts) -> anyhow::Result<()> {
    let root = contexts
        .get("user")
        .and_then(|user| user.get("id"))
        .is_some_and(|id| id.to_string() == "0");

    if root {
        return Err(anyhow!(
            "{provider} can't run as root. Run comtrya as a normal user, it asks for sudo where needed, or use `provider: pacman` for packages from the official repositories"
        ));
    }

    Ok(())
}

/// The arguments as owned strings, for `Exec`
fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{fails, succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    fn with_runner(helper: &'static Helper, runner: FakeRunner) -> AurHelper {
        AurHelper {
            helper,
            runner: runner.shared(),
        }
    }

    fn package(list: &[&str]) -> PackageVariant {
        PackageVariant {
            list: strings(list),
            ..Default::default()
        }
    }

    fn user_contexts(id: &str) -> Contexts {
        let mut contexts = Contexts::default();
        contexts.insert(
            String::from("user"),
            [(String::from("id"), id.into())].into_iter().collect(),
        );
        contexts
    }

    #[test]
    fn query_asks_pacman_what_is_installed() {
        let runner = FakeRunner::new()
            .answer(
                "pacman -Q -q git yay-bin",
                fails("git\n", "error: package 'yay-bin' was not found\n"),
            )
            .shared();
        let yay = AurHelper {
            helper: &YAY,
            runner: runner.clone(),
        };

        assert_eq!(
            vec!["yay-bin"],
            yay.query(&package(&["git", "yay-bin"])).unwrap()
        );
        assert_eq!(vec!["pacman -Q -q git yay-bin"], runner.calls());
    }

    #[test]
    fn install_skips_installed_packages() {
        for (helper, expected) in [
            (&YAY, "yay -S --noconfirm yay-bin"),
            (&PARU, "paru -Sq --batchinstall --needed --noconfirm --noprogressbar --skipreview --sudoloop --useask yay-bin"),
        ] {
            let runner = FakeRunner::new().answer(
                "pacman -Q -q git yay-bin",
                fails("git\n", "error: package 'yay-bin' was not found\n"),
            );

            let steps = with_runner(helper, runner)
                .install(&package(&["git", "yay-bin"]), &Contexts::default())
                .unwrap();

            assert_eq!(1, steps.len());
            assert_eq!(
                format!("CommandExec with: privileged=false: {expected}"),
                steps[0].atom.to_string()
            );
        }
    }

    #[test]
    fn install_does_nothing_when_everything_is_installed() {
        let runner = FakeRunner::new().answer("pacman -Q -q git", succeeds("git\n"));

        let steps = with_runner(&PARU, runner)
            .install(&package(&["git"]), &Contexts::default())
            .unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn bootstrap_builds_the_helper_from_the_aur() {
        let steps = AurHelper::paru().bootstrap(&Contexts::default());
        let commands: Vec<String> = steps.iter().map(|step| step.atom.to_string()).collect();

        assert_eq!(3, commands.len());
        assert!(commands[0].ends_with("pacman -S --needed --noconfirm base-devel git"));
        assert!(commands[1].ends_with("git clone https://aur.archlinux.org/paru.git /tmp/paru"));
        assert!(commands[2].ends_with("makepkg -si --noconfirm"));
    }

    #[test]
    fn aur_helpers_refuse_to_run_as_root() {
        for helper in [AurHelper::yay(), AurHelper::paru()] {
            let err = match helper.install(&package(&["yay-only-package"]), &user_contexts("0")) {
                Ok(_) => panic!("{} installed as root", helper.name()),
                Err(err) => err.to_string(),
            };
            assert!(err.contains("can't run as root"), "{err}");
            assert!(err.contains("provider: pacman"), "{err}");
        }
    }

    #[test]
    fn root_is_detected_by_uid() {
        assert!(refuse_root("yay", &user_contexts("0")).is_err());
        assert!(refuse_root("yay", &user_contexts("1000")).is_ok());
        assert!(refuse_root("yay", &Contexts::default()).is_ok());
    }
}
