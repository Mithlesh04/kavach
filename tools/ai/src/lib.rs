//! Tools that let AI assistants ask the compiler instead of reading files
//! (spec chapter 13, decision 0015): `outline`, `query` and compact
//! machine-readable messages. `query` and `context` answer from the
//! program's index, which a project keeps in `.kavach/index`.

mod context;
mod index;
mod json;
mod mcp;
mod outline;
mod query;
mod store;

pub use context::{context, estimate_tokens};
pub use index::{Decl, Index, Ref};
pub use json::messages_json;
pub use mcp::serve;
pub use outline::{outline, outline_item};
pub use query::query;
pub use store::{NoIndex, index_for};
