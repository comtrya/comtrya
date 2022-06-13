use anyhow::anyhow;
use comtrya_plugins::{Function, InvocationError, PluginDefinition};
use libloading::Library;
use std::{collections::HashMap, ffi::OsStr, path::PathBuf, rc::Rc};
use tracing::{debug, info, trace};

#[derive(Clone)]
pub struct FunctionProxy {
    pub(super) function: Box<dyn Function>,
    _lib: Rc<Library>,
}

impl Function for FunctionProxy {
    fn plan(&self, arguments: &[String]) -> Result<bool, InvocationError> {
        self.function.plan(arguments)
    }

    fn run(&self, arguments: &[String]) -> Result<(), InvocationError> {
        self.function.run(arguments)
    }
}

#[derive(Default)]
pub struct PluginFunctions {
    pub(super) functions: HashMap<String, FunctionProxy>,
    libraries: Vec<Rc<Library>>,
}

impl PluginFunctions {
    pub fn new() -> PluginFunctions {
        PluginFunctions::default()
    }

    /// Load a plugin library and add all contained functions to the internal
    /// function table.
    ///
    /// # Safety
    ///
    /// A plugin library **must** be implemented using the
    /// [`comtrya_plugins::plugin_def!()`] macro. Trying manually implement
    /// a plugin without going through that macro will result in undefined
    /// behaviour.
    pub unsafe fn load<P: AsRef<OsStr>>(&mut self, library_path: P) -> anyhow::Result<()> {
        let library = Rc::new(Library::new(library_path)?);

        let definition = library
            .get::<*mut PluginDefinition>(b"plugin_definition\0")?
            .read();

        if definition.rustc_version != comtrya_plugins::RUSTC_VERSION
            || definition.core_version != comtrya_plugins::CORE_VERSION
        {
            return Err(anyhow!(
                "Plugin version mismatch: rustc_version={}, core_version={}",
                definition.rustc_version,
                definition.core_version
            ));
        }

        let mut registrar = PluginRegistrar::new(Rc::clone(&library));

        (definition.register)(&mut registrar);

        self.functions.extend(registrar.functions);

        self.libraries.push(library);

        debug!("Registered functions: {:?}", self.functions.keys());

        Ok(())
    }
}

pub fn load_plugin_functions(library_path: &String) -> anyhow::Result<PluginFunctions> {
    let mut plugin_functions = PluginFunctions::new();

    if let Ok(library_dir) = PathBuf::from(library_path).read_dir() {
        for entry in library_dir.flatten() {
            let path = entry.path();

            unsafe {
                match plugin_functions.load(&path) {
                    Ok(_) => (),
                    Err(e) => {
                        trace!("Failed to load plugin {}: {}", path.display(), e);
                    }
                }
            }
        }
    } else {
        info!(
            "Cannot read plugins directory: {}. No plugins are available!",
            library_path
        );
    }

    Ok(plugin_functions)
}

struct PluginRegistrar {
    functions: HashMap<String, FunctionProxy>,
    lib: Rc<Library>,
}

impl PluginRegistrar {
    pub fn new(lib: Rc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            functions: HashMap::default(),
            lib,
        }
    }
}

impl comtrya_plugins::PluginRegistrar for PluginRegistrar {
    fn register_function(&mut self, name: &str, function: Box<dyn Function>) {
        let proxy = FunctionProxy {
            function,
            _lib: Rc::clone(&self.lib),
        };

        self.functions.insert(name.to_string(), proxy);
    }
}
