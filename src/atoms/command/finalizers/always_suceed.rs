use super::Finalizer;

#[derive(Clone, Debug)]
pub struct AlwaysSuceed();

impl Finalizer for AlwaysSuceed {
    fn run(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_returns_true() {
        let always_suceed = AlwaysSuceed {};

        assert_eq!(true, always_suceed.run())
    }
}
