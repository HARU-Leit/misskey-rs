//! Federation middleware for signature verification and security.

mod signature_verification;

pub use signature_verification::{
    SignatureVerificationLayer, SignatureVerificationState, SignatureVerified,
};
