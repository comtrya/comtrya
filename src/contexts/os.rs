use crate::contexts::{Context, ContextProvider};
use os_info;

pub struct OSContextProvider {}

impl ContextProvider for OSContextProvider {
    fn get_prefix(&self) -> String {
        String::from("os")
    }

    fn get_contexts(&self) -> Vec<super::Context> {
        let osinfo = os_info::get();

        vec![
            Context::KeyValueContext(String::from("name"), format!("{}", osinfo.os_type())),
            Context::KeyValueContext(
                String::from("codename"),
                String::from(osinfo.codename().unwrap_or("unknown")),
            ),
            Context::KeyValueContext(String::from("bitness"), format!("{}", osinfo.bitness())),
            Context::KeyValueContext(String::from("version"), format!("{}", osinfo.version())),
            Context::KeyValueContext(
                String::from("edition"),
                String::from(osinfo.edition().unwrap_or("unknown")),
            ),
        ]
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
        let os = os_info::get();

        if format!("{}", os.os_type()).eq("Mac OS") {
            let oscontext = OSContextProvider {};
            let keyvaluepairs = oscontext.get_contexts();

            keyvaluepairs.iter().for_each(|context| match context {
                Context::KeyValueContext(k, v) => {
                    if k.eq("name") {
                        assert_eq!(v, &String::from("Mac OS"));
                    }
                    if k.eq("bitness") {
                        assert_eq!(v, &String::from("64-bit"));
                    }
                }
                Context::ListContext(_, _) => {
                    assert_eq!(true, false);
                }
            })
        }
    }

    #[test]
    #[cfg(windows)]
    fn it_can_windows() {
        let oscontext = OSContextProvider {};
        let keyvaluepairs = oscontext.get_contexts();

        keyvaluepairs.iter().for_each(|context| match context {
            Context::KeyValueContext(k, v) => {
                if k.eq("name") {
                    assert_eq!(v, &String::from("Windows"));
                }
                if k.eq("bitness") {
                    assert_eq!(v, &String::from("64-bit"));
                }
            }
            Context::ListContext(_, _) => {
                assert_eq!(true, false);
            }
        })
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn it_can_linux() {
        let os = os_info::get();

        if format!("{}", os.os_type()).eq("linux") {
            let oscontext = OSContextProvider {};
            let keyvaluepairs = oscontext.get_contexts();

            keyvaluepairs.iter().for_each(|context| match context {
                Context::KeyValueContext(k, v) => {
                    if k.eq("name") {
                        assert_eq!(v, &String::from("Ubuntu"));
                    }
                    if k.eq("bitness") {
                        assert_eq!(v, &String::from("64-bit"));
                    }
                }
                Context::ListContext(_, _) => {
                    assert_eq!(true, false);
                }
            })
        }
    }
}
