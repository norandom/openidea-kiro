//! Files by component and by task coverage views (tasks 2.4, 2.5).

use crate::args::Request;
use crate::context::CommandContext;
use crate::present::Output;

pub fn by_component(_ctx: &mut CommandContext<'_>, _req: &Request) -> Result<Output, String> {
    Err("not implemented".into())
}

pub fn by_task(_ctx: &mut CommandContext<'_>, _req: &Request) -> Result<Output, String> {
    Err("not implemented".into())
}
