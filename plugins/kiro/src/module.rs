//! Feature module contract (task 3.2).

use crate::args::Request;
use crate::context::CommandContext;
use crate::present::Output;
use crate::registry::{CommandSpec, Group};

/// Contract implemented by every command group module.
pub trait FeatureModule {
    /// Command group owned by this module.
    fn group(&self) -> Group;

    /// Static declarations of base commands in this group.
    fn commands(&self) -> &'static [CommandSpec];

    /// Executes a command belonging to this group.
    fn run(
        &self,
        ctx: &mut CommandContext<'_>,
        cmd: &CommandSpec,
        req: &Request,
    ) -> Result<Output, String>;
}
