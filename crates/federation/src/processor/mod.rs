//! Activity processors for handling incoming `ActivityPub` activities.

#![allow(missing_docs)]

mod accept;
mod actor_fetcher;
mod announce;
mod create;
mod delete;
mod emoji_react;
mod follow;
mod like;
mod move_processor;
mod reject;
mod undo;
mod update;

pub use accept::AcceptProcessor;
pub use actor_fetcher::ActorFetcher;
pub use announce::AnnounceProcessor;
pub use create::CreateProcessor;
pub use delete::{DeleteProcessor, DeleteResult};
pub use emoji_react::EmojiReactProcessor;
pub use follow::{FollowProcessResult, FollowProcessor};
pub use like::LikeProcessor;
pub use move_processor::{MoveProcessResult, MoveProcessor};
pub use reject::RejectProcessor;
pub use undo::{ParsedUndoActivity, UndoProcessor, UndoResult};
pub use update::{UpdateProcessor, UpdateResult};
