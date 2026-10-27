//! Steps: the atoms an action plans, with the checks around them

use crate::atoms::Atom;
use crate::steps::finalizers::FlowControl;
use tracing::error;

pub mod finalizers;
pub mod initializers;

/// One change to the system: an atom, guarded by initializers that decide whether it runs and
/// finalizers that decide whether the action goes on after it
pub struct Step {
    pub atom: Box<dyn Atom>,
    /// Checked before the atom runs, see [`Step::do_initializers_allow_us_to_run`]
    pub initializers: Vec<initializers::FlowControl>,
    /// Checked after the atom ran, see [`Step::do_finalizers_allow_us_to_continue`]
    pub finalizers: Vec<finalizers::FlowControl>,
}

impl Step {
    /// A step that runs `atom` without initializers or finalizers
    pub fn new(atom: impl Atom + 'static) -> Self {
        Step {
            atom: Box::new(atom),
            initializers: vec![],
            finalizers: vec![],
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

    /// Every finalizer has to allow continuing. All of them run, even after one said stop,
    /// because finalizers also clean up, like removing env vars again
    pub fn do_finalizers_allow_us_to_continue(&self) -> bool {
        self.finalizers
            .iter()
            .map(|flow_control| match flow_control {
                // Stop if true
                FlowControl::StopIf(i) => match i.finalize(self.atom.as_ref()) {
                    Ok(stop) => !stop,
                    Err(err) => {
                        error!("Failed to run finalizers: {}", err.to_string());

                        // On an error, we can't really determine if this Atom should
                        // run; so lets play it safe and filter it out too
                        false
                    }
                },
                FlowControl::Ensure(i) => match i.finalize(self.atom.as_ref()) {
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

        assert_eq!(true, step.do_finalizers_allow_us_to_continue());

        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(true))));

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());

        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false))))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(true))));

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());
    }

    #[test]
    fn finalizers_that_error_block_execution() {
        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(ErrorFinalizer())));

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());

        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(EchoFinalizer(false))))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(ErrorFinalizer())));

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());
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
        assert_eq!(false, step.do_finalizers_allow_us_to_continue());

        let step = Step::new(EchoAtom("hello-world"))
            .with_finalizer(FinalizerFlowControl::StopIf(Box::new(ErrorFinalizer())))
            .with_finalizer(FinalizerFlowControl::Ensure(Box::new(EchoFinalizer(true))));
        assert_eq!(false, step.do_finalizers_allow_us_to_continue());
    }

    /// Counts how often it ran, and allows continuing as told
    struct CountingFinalizer(std::rc::Rc<std::cell::Cell<u32>>, bool);

    impl finalizers::Finalizer for CountingFinalizer {
        fn finalize(&self, _atom: &dyn crate::atoms::Atom) -> anyhow::Result<bool> {
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

        assert_eq!(false, step.do_finalizers_allow_us_to_continue());
        assert_eq!(2, runs.get());
    }
}
