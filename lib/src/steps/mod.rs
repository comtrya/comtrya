use crate::atoms::Atom;
use crate::steps::finalizers::FlowControl;
use tracing::error;

pub mod finalizers;
pub mod initializers;

pub struct Step {
    pub atom: Box<dyn Atom>,
    pub initializers: Vec<initializers::FlowControl>,
    pub finalizers: Vec<finalizers::FlowControl>,
}

impl Step {
    pub fn do_initializers_allow_us_to_run(&self) -> bool {
        self.initializers
            .iter()
            .fold(true, |_, flow_control| match flow_control {
                initializers::FlowControl::Ensure(i) => i.initialize().unwrap_or_else(|err| {
                    error!("Failed to run initializer: {}", err.to_string());
                    false
                }),

                initializers::FlowControl::SkipIf(i) => match i.initialize() {
                    Ok(true) => false,
                    Ok(false) => true,
                    Err(err) => {
                        error!("Failed to run initializer: {}", err.to_string());
                        false
                    }
                },
            })
    }

    pub fn do_finalizers_allow_us_to_continue(&mut self) -> bool {
        self.finalizers
            .iter()
            .fold(true, |_, flow_control| match flow_control {
                finalizers::FlowControl::StopIf(i) => match i.finalize(self.atom.as_ref()) {
                    Ok(true) => false,
                    Ok(false) => true,
                    Err(err) => {
                        error!("Failed to run finalizers: {}", err.to_string());
                        false
                    }
                },
                FlowControl::Ensure(i) => match i.finalize(self.atom.as_ref()) {
                    Ok(true) => true,
                    Ok(false) => false,
                    Err(err) => {
                        error!("Failed to run finalizers: {}", err.to_string());
                        false
                    }
                },
            })
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
        let step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![InitializerFlowControl::Ensure(Box::new(EchoInitializer(
                true,
            )))],
            finalizers: vec![],
        };

        assert_eq!(true, step.do_initializers_allow_us_to_run());

        let step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![InitializerFlowControl::Ensure(Box::new(EchoInitializer(
                false,
            )))],
            finalizers: vec![],
        };

        assert_eq!(false, step.do_initializers_allow_us_to_run());

        let step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![InitializerFlowControl::SkipIf(Box::new(EchoInitializer(
                true,
            )))],
            finalizers: vec![],
        };

        assert_eq!(false, step.do_initializers_allow_us_to_run());

        let step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![InitializerFlowControl::SkipIf(Box::new(EchoInitializer(
                false,
            )))],
            finalizers: vec![],
        };

        assert_eq!(true, step.do_initializers_allow_us_to_run());

        let step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![
                InitializerFlowControl::SkipIf(Box::new(EchoInitializer(false))),
                InitializerFlowControl::SkipIf(Box::new(EchoInitializer(true))),
            ],
            finalizers: vec![],
        };

        assert_eq!(false, step.do_initializers_allow_us_to_run());
    }

    #[test]
    fn initializers_that_error_block_execution() {
        let step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![InitializerFlowControl::SkipIf(Box::new(ErrorInitializer()))],
            finalizers: vec![],
        };

        assert_eq!(false, step.do_initializers_allow_us_to_run());

        let step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![
                InitializerFlowControl::SkipIf(Box::new(EchoInitializer(false))),
                InitializerFlowControl::SkipIf(Box::new(ErrorInitializer())),
            ],
            finalizers: vec![],
        };

        assert_eq!(false, step.do_initializers_allow_us_to_run());
    }

    #[test]
    fn finalizers_can_control_execution() {
        let mut step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![],
            finalizers: vec![FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false)))],
        };

        assert_eq!(true, step.do_finalizers_allow_us_to_continue());

        let mut step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![],
            finalizers: vec![FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(true)))],
        };

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());

        let mut step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![],
            finalizers: vec![
                FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false))),
                FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(true))),
            ],
        };

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());
    }

    #[test]
    fn finalizers_that_error_block_execution() {
        let mut step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![],
            finalizers: vec![FinalizerFlowControl::StopIf(Box::new(ErrorFinalizer()))],
        };

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());

        let mut step = Step {
            atom: Box::new(EchoAtom("hello-world")),
            initializers: vec![],
            finalizers: vec![
                FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false))),
                FinalizerFlowControl::StopIf(Box::new(ErrorFinalizer())),
            ],
        };

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());
    }
}
