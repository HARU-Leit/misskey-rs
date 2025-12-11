//! Job definitions.

#![allow(missing_docs)]

mod deliver;
mod inbox;

pub use deliver::DeliverJob;
pub use inbox::InboxJob;
