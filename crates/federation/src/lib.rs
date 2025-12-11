//! ActivityPub federation for misskey-rs.
//!
//! This crate implements the ActivityPub protocol for federated social networking:
//!
//! - **Activities**: Create, Delete, Follow, Like, Announce, Update, Undo
//! - **Actors**: Person actor implementation
//! - **Objects**: Note, Question, Image objects
//! - **Handlers**: WebFinger, NodeInfo, inbox/outbox endpoints
//! - **Security**: HTTP signatures, replay protection, rate limiting
//! - **Delivery**: Activity delivery with retry and dead letter queue
//!
//! # ActivityPub Compliance
//!
//! This implementation follows the W3C ActivityPub specification with
//! Misskey-specific extensions prefixed with `_misskey_`.

pub mod activities;
pub mod actors;
pub mod cache;
pub mod client;
pub mod convert;
pub mod delivery;
pub mod handler;
pub mod objects;
pub mod processor;
pub mod security;
pub mod signature;

pub use activities::*;
pub use actors::*;
pub use cache::{CacheError, CacheStats, CachedRemoteActor, RemoteActorCache};
pub use client::{ApClient, ApClientError};
pub use convert::*;
pub use delivery::DeliveryService;
pub use handler::*;
pub use objects::*;
pub use processor::{
    AcceptProcessor, ActorFetcher, AnnounceProcessor, CreateProcessor, DeleteProcessor,
    DeleteResult, EmojiReactProcessor, FollowProcessResult, FollowProcessor, LikeProcessor,
    ParsedUndoActivity, RejectProcessor, UndoProcessor, UndoResult, UpdateProcessor, UpdateResult,
};
pub use security::{
    ActivitySecurityChecker, FederationRateLimiter, RateLimitError, RateLimitStatus, ReplayError,
    ReplayProtection, SecurityCheckResult, SecurityError,
};
pub use signature::{HttpSigner, HttpVerifier, SignatureComponents, SignatureError};
