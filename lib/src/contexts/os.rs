use crate::contexts::{Context, ContextProvider};
use anyhow::Result;
use gethostname::gethostname;
use os_info;

pub struct OSContextProvider {}

impl ContextProvider for OSContextProvider {
    fn get_prefix(&self) -> String {
        String::from("os")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        let osinfo = os_info::get();

        Ok(vec![
            Context::KeyValueContext(String::from("hostname"), gethostname().into()),
            Context::KeyValueContext(String::from("family"), std::env::consts::FAMILY.into()),
            Context::KeyValueContext(String::from("name"), std::env::consts::OS.into()),
            Context::KeyValueContext(
                String::from("distribution"),
                format!("{}", osinfo.os_type()).into(),
            ),
            Context::KeyValueContext(
                String::from("codename"),
                String::from(osinfo.codename().unwrap_or("unknown")).into(),
            ),
            Context::KeyValueContext(
                String::from("bitness"),
                format!("{}", osinfo.bitness()).into(),
            ),
            Context::KeyValueContext(
                String::from("version"),
                format!("{}", osinfo.version()).into(),
            ),
            Context::KeyValueContext(
                String::from("edition"),
                String::from(osinfo.edition().unwrap_or("unknown")).into(),
            ),
        ])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_can_prefix() {
        let oscontext = OSContextProvider {};
        let prefix = oscontext.get_prefix();
        assert_eq!(String::from("os"), prefix);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn it_can_macos() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => match k.as_ref() {
                "family" => assert_eq!(v.to_string(), String::from("unix")),
                "name" => assert_eq!(v.to_string(), String::from("macos")),
                _ => (),
            },
            Context::ListContext(_, _) => {
                assert_eq!(true, false);
            }
        })
    }

    #[test]
    #[cfg(windows)]
    fn it_can_windows() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => match k.as_ref() {
                "family" => assert_eq!(v.to_string(), String::from("windows")),
                "name" => assert_eq!(v.to_string(), String::from("windows")),
                _ => (),
            },
            Context::ListContext(_, _) => {
                assert_eq!(true, false);
            }
        })
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn it_can_linux() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => match k.as_ref() {
                "family" => assert_eq!(v.to_string(), String::from("unix")),
                "name" => assert_eq!(v.to_string(), String::from("linux")),
                _ => (),
            },
            Context::ListContext(_, _) => {
                assert_eq!(true, false);
            }
        })
    }

    #[test]
    #[cfg(target_os = "freebsd")]
    fn it_can_linux() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts().unwrap();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => match k.as_ref() {
                "family" => assert_eq!(v.to_string(), String::from("unix")),
                "name" => assert_eq!(v.to_string(), String::from("freebsd")),
                _ => (),
            },
            Context::ListContext(_, _) => {
                assert_eq!(true, false);
            }
        })
    }
}
