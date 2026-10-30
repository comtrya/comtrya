//! Steps: the atoms an action plans, with the checks around them

use crate::atoms::{self, Atom, Output, RunAs};
use crate::contexts::privilege::Privilege;
use crate::steps::finalizers::FlowControl;
use crate::utilities;
use tracing::error;

/// Checks after an atom ran
pub mod finalizers;
/// Checks before an atom runs
pub mod initializers;

/// One change to the system: an atom, guarded by initializers that decide whether it runs and
/// finalizers that decide whether the action goes on after it
pub struct Step {
    /// The change to make
    pub atom: Atom,
    /// Who makes it
    pub run_as: RunAs,
    /// Checked before the atom runs, see [`Step::do_initializers_allow_us_to_run`]
    pub initializers: Vec<initializers::FlowControl>,
    /// Checked after the atom ran, see [`Step::do_finalizers_allow_us_to_continue`]
    pub finalizers: Vec<finalizers::FlowControl>,
}

impl Step {
    /// A step that runs `atom` without initializers or finalizers
    pub fn new(atom: impl Into<Atom>) -> Self {
        Step {
            atom: atom.into(),
            run_as: RunAs::User,
            initializers: vec![],
            finalizers: vec![],
        }
    }

    /// Runs the atom as root
    pub fn as_root(mut self) -> Self {
        self.run_as = RunAs::Root;
        self
    }

    /// The atom to execute: a root step runs through `privilege`, unless comtrya runs as root
    /// already
    pub fn atom_to_run(&self, privilege: &Privilege) -> atoms::Result<Atom> {
        self.atom_to_run_with(privilege, utilities::running_as_root())
    }

    /// [`Step::atom_to_run`], with whether comtrya runs as root
    fn atom_to_run_with(&self, privilege: &Privilege, as_root: bool) -> atoms::Result<Atom> {
        match self.run_as {
            RunAs::Root if !as_root => self.atom.clone().elevated(privilege),
            _ => Ok(self.atom.clone()),
        }
    }

    /// Adds an initializer, checked before the atom runs
    pub fn with_initializer(mut self, initializer: initializers::FlowControl) -> Self {
        self.initializers.push(initializer);
        self
    }

    /// Adds a finalizer, checked after the atom ran
    pub fn with_finalizer(mut self, finalizer: finalizers::FlowControl) -> Self {
        self.finalizers.push(finalizer);
        self
    }

    /// Every initializer has to allow the step. Stops at the first one that doesn't, so later
    /// initializers with side effects, like setting env vars, don't run for a skipped step
    pub fn do_initializers_allow_us_to_run(&self) -> bool {
        self.initializers
            .iter()
            .all(|flow_control| match flow_control {
                initializers::FlowControl::Ensure(i) => i.initialize().unwrap_or_else(|err| {
                    error!("Failed to run initializer: {}", err.to_string());

                    // On an error, we can't really determine if this Atom should
                    // run; so lets play it safe and filter it out too
                    false
                }),

                // Skip if true
                initializers::FlowControl::SkipIf(i) => match i.initialize() {
                    Ok(skip) => !skip,
                    Err(err) => {
                        error!("Failed to run initializer: {}", err.to_string());

                        // On an error, we can't really determine if this Atom should
                        // run; so lets play it safe and filter it out too
                        false
                    }
                },
            })
    }

    /// Every finalizer has to allow continuing, judging the `output` of the atom. All of them
    /// run, even after one said stop, because finalizers also clean up, like removing env vars
    /// again
    pub fn do_finalizers_allow_us_to_continue(&self, output: &Output) -> bool {
        self.finalizers
            .iter()
            .map(|flow_control| match flow_control {
                // Stop if true
                FlowControl::StopIf(i) => match i.finalize(output) {
                    Ok(stop) => !stop,
                    Err(err) => {
                        error!("Failed to run finalizers: {}", err.to_string());

                        // On an error, we can't really determine if this Atom should
                        // run; so lets play it safe and filter it out too
                        false
                    }
                },
                FlowControl::Ensure(i) => match i.finalize(output) {
                    Ok(ensured) => ensured,
                    Err(err) => {
                        error!("Failed to run finalizers: {}", err.to_string());

                        // On an error, we can't really determine if this Atom should
                        // run; so lets play it safe and filter it out too
                        false
                    }
                },
            })
            // Collected first, so every finalizer runs before `all` short-circuits
            .collect::<Vec<bool>>()
            .into_iter()
            .all(|allowed| allowed)
    }
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.run_as {
            RunAs::User => write!(f, "{}", self.atom),
            RunAs::Root => write!(f, "{} (as root)", self.atom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::finalizers::test::EchoFinalizer;
    use super::finalizers::test::ErrorFinalizer;
    use super::finalizers::FlowControl as FinalizerFlowControl;
    use super::initializers::test::Echo as EchoInitializer;
    use super::initializers::test::Error as ErrorInitializer;
    use super::initializers::FlowControl as InitializerFlowControl;
    use crate::atoms::Echo as EchoAtom;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn initializers_can_control_execution() {
        let step = Step::new(EchoAtom("hello-world")).with_initializer(
            InitializerFlowControl::Ensure(Box::new(EchoInitializer(true))),
        );

        assert_eq!(true, step.do_initializers_allow_us_to_run());

        let step = Step::new(EchoAtom("hello-world")).with_initializer(
            InitializerFlowControl::Ensure(Box::new(EchoInitializer(false))),
        );

        assert_eq!(false, step.do_initializers_allow_us_to_run());

        let step = Step::new(EchoAtom("hello-world")).with_initializer(
            InitializerFlowControl::SkipIf(Box::new(EchoInitializer(true))),
        );

        assert_eq!(false, step.do_initializers_allow_us_to_run());

        let step = Step::new(EchoAtom("hello-world")).with_initializer(
            InitializerFlowControl::SkipIf(Box::new(EchoInitializer(false))),
        );

        assert_eq!(true, step.do_initializers_allow_us_to_run());

        let step = Step::new(EchoAtom("hello-world"))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(EchoInitializer(
                false,
            ))))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(EchoInitializer(
                true,
            ))));

        assert_eq!(false, step.do_initializers_allow_us_to_run());
    }

    #[test]
    fn initializers_that_error_block_execution() {
        let step = Step::new(EchoAtom("hello-world"))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(ErrorInitializer())));

        assert_eq!(false, step.do_initializers_allow_us_to_run());

        let step = Step::new(EchoAtom("hello-world"))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(EchoInitializer(
                false,
            ))))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(ErrorInitializer())));

        assert_eq!(false, step.do_initializers_allow_us_to_run());
    }

    #[test]
    fn finalizers_can_control_execution() {
        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false))));

        assert_eq!(
            true,
            step.do_finalizers_allow_us_to_continue(&Output::default())
        );

        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(true))));

        assert_eq!(
            false,
            step.do_finalizers_allow_us_to_continue(&Output::default())
        );

        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false))))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(true))));

        assert_eq!(
            false,
            step.do_finalizers_allow_us_to_continue(&Output::default())
        );
    }

    #[test]
    fn finalizers_that_error_block_execution() {
        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(ErrorFinalizer())));

        assert_eq!(
            false,
            step.do_finalizers_allow_us_to_continue(&Output::default())
        );

        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false))))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(ErrorFinalizer())));

        assert_eq!(
            false,
            step.do_finalizers_allow_us_to_continue(&Output::default())
        );
    }

    #[test]
    fn any_initializer_can_block_execution() {
        // The blocking initializer comes first, a later one that allows must not win
        let step = Step::new(EchoAtom("hello-world"))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(EchoInitializer(
                true,
            ))))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(EchoInitializer(
                false,
            ))));
        assert_eq!(false, step.do_initializers_allow_us_to_run());

        let step = Step::new(EchoAtom("hello-world"))
            .with_initializer(InitializerFlowControl::Ensure(Box::new(EchoInitializer(
                false,
            ))))
            .with_initializer(InitializerFlowControl::Ensure(Box::new(EchoInitializer(
                true,
            ))));
        assert_eq!(false, step.do_initializers_allow_us_to_run());

        let step = Step::new(EchoAtom("hello-world"))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(ErrorInitializer())))
            .with_initializer(InitializerFlowControl::SkipIf(Box::new(EchoInitializer(
                false,
            ))));
        assert_eq!(false, step.do_initializers_allow_us_to_run());
    }

    #[test]
    fn any_finalizer_can_stop_execution() {
        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(true))))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false))));
        assert_eq!(
            false,
            step.do_finalizers_allow_us_to_continue(&Output::default())
        );

        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(ErrorFinalizer())))
            .with_finalizer(FinalizerFlowControl::Ensure(Box::new(EchoFinalizer(true))));
        assert_eq!(
            false,
            step.do_finalizers_allow_us_to_continue(&Output::default())
        );
    }

    /// Counts how often it ran, and allows continuing as told
    struct CountingFinalizer(std::rc::Rc<std::cell::Cell<u32>>, bool);

    impl finalizers::Finalizer for CountingFinalizer {
        fn finalize(&self, _output: &Output) -> anyhow::Result<bool> {
            self.0.set(self.0.get() + 1);
            Ok(self.1)
        }
    }

    #[test]
    fn all_finalizers_run_after_one_stops() {
        let runs = std::rc::Rc::new(std::cell::Cell::new(0));
        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::Ensure(Box::new(CountingFinalizer(
                runs.clone(),
                false,
            ))))
            .with_finalizer(FinalizerFlowControl::Ensure(Box::new(CountingFinalizer(
                runs.clone(),
                true,
            ))));

        assert_eq!(
            false,
            step.do_finalizers_allow_us_to_continue(&Output::default())
        );
        assert_eq!(2, runs.get());
    }

    /// `echo hi`, as root
    fn root_echo() -> Step {
        let mut exec = crate::atoms::command::Exec {
            command: String::from("echo"),
            ..Default::default()
        };
        exec.arguments = vec![String::from("hi")];
        Step::new(exec).as_root()
    }

    #[test]
    fn a_root_step_runs_through_the_privilege_provider() {
        let atom = root_echo()
            .atom_to_run_with(&Privilege::Doas, false)
            .unwrap();

        assert_eq!("CommandExec with: doas echo hi", atom.to_string());
    }

    /// Root is found by its uid, so also under another name or without a passwd entry
    #[test]
    fn a_root_step_runs_as_it_is_when_comtrya_is_root() {
        let atom = root_echo()
            .atom_to_run_with(&Privilege::Doas, true)
            .unwrap();

        assert_eq!("CommandExec with: echo hi", atom.to_string());
    }

    #[test]
    fn a_user_step_runs_as_it_is() {
        let step = Step::new(crate::atoms::command::Exec {
            command: String::from("id"),
            ..Default::default()
        });

        assert_eq!(
            "CommandExec with: id ",
            step.atom_to_run_with(&Privilege::Sudo, false)
                .unwrap()
                .to_string()
        );
    }

    #[test]
    fn a_root_step_says_so() {
        assert_eq!(
            "CommandExec with: echo hi (as root)",
            root_echo().to_string()
        );
    }
}
