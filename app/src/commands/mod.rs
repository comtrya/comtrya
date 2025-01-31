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
    async fn execute(&self, runtime: &mut Runtime) -> anyhow::Result<()>;
}
