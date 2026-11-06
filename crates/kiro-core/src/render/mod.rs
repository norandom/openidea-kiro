//! The generic graph model and the machinery shared by both renderers:
//! `Graph` with its nodes and edges, direction, size caps, safe unique
//! identifiers, the rendered result with its truncation flag, and line
//! clipping under a byte limit.
//!
//! Output formats live in the two submodules, declared up front so the crate
//! builds before either renderer has content: `mermaid` emits Mermaid graphs
//! and `text` emits plain-text graphs, finding lines and Markdown tables.
//! Design components `RenderGraph`, `MermaidRenderer` and `TextRenderer`;
//! this stub is intentionally empty until those components are implemented.

pub mod mermaid;
pub mod text;
