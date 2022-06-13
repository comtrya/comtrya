use dyn_clone::DynClone;

pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

dyn_clone::clone_trait_object!(Function);

pub trait Function: DynClone {
    fn plan(&self, arguments: &[String]) -> Result<bool, InvocationError>;
    fn run(&self, arguments: &[String]) -> Result<(), InvocationError>;
}

pub enum InvocationError {
    InvalidArgumentCount { expected: usize, actual: usize },
    Other { message: String },
}

pub trait PluginRegistrar {
    fn register_function(&mut self, name: &str, function: Box<dyn Function>);
}

pub struct PluginDefinition {
    pub rustc_version: &'static str,
    pub core_version: &'static str,
    pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
}

#[macro_export]
macro_rules! export_plugin {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static plugin_definition: $crate::PluginDefinition = $crate::PluginDefinition {
            rustc_version: $crate::RUSTC_VERSION,
            core_version: $crate::CORE_VERSION,
            register: $register,
        };
    };
}
