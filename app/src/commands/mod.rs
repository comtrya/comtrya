mod apply;
pub(crate) use apply::Apply;

mod version;
pub(crate) use version::Version;

mod contexts;
pub(crate) use contexts::Contexts;

use crate::Runtime;

pub trait ComtryaCommand {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()>;
}
