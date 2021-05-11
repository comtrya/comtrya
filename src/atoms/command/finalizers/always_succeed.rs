use super::Finalizer;

#[derive(Clone, Debug)]
pub struct AlwaysSucceed();

impl Finalizer for AlwaysSucceed {
    fn run(&self, _: &Result<std::process::Output, std::io::Error>) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_returns_true() {
        let err = Err(std::io::Error::new(std::io::ErrorKind::NotFound, ""));
        let always_succeed = AlwaysSucceed {};

        assert_eq!(true, always_succeed.run(&err));
    }
}
