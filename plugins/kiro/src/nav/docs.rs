//! Outline views for requirements, design, and tasks documents (task 2.6).

use crate::args::Request;
use crate::context::CommandContext;
use crate::present::Output;
use kiro_core::spec::Doc;

pub fn run(_ctx: &mut CommandContext<'_>, _req: &Request, _doc: Doc) -> Result<Output, String> {
    Err("not implemented".into())
}
