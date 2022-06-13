use comtrya_plugins::{Function, InvocationError, PluginRegistrar};

#[derive(Clone)]
pub struct Example;

impl Function for Example {
    fn plan(&self, arguments: &[String]) -> Result<bool, InvocationError> {
        println!("Example plugin: Planning! Arguments: {:?}", arguments);

        Ok(true)
    }

    fn run(&self, _arguments: &[String]) -> Result<(), InvocationError> {
        println!("Example plugin: Running! Arguments: {:?}", _arguments);

        Ok(())
    }
}

comtrya_plugins::export_plugin!(register);

extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_function("example", Box::new(Example));
}
