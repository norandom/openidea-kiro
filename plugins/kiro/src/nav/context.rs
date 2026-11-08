//! Spec context view and reference line generation (tasks 2.2, 2.3).

use crate::args::Request;
use crate::context::CommandContext;
use crate::present::Output;

pub fn run(_ctx: &mut CommandContext<'_>, _req: &Request) -> Result<Output, String> {
    Err("not implemented".into())
}
