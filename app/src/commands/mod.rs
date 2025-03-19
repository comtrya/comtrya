mod apply;
pub(crate) use apply::Apply;

mod version;
pub(crate) use version::Version;

mod contexts;
pub(crate) use contexts::Contexts;

mod gen_completions;
pub(crate) use gen_completions::GenCompletions;

use crate::Runtime;

pub trait ComtryaCommand {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()>;
}
