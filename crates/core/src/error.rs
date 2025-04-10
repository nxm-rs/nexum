//! Default error type for APDU operations
use crate::{
    processor::{ProcessorError, SecureProtocolError},
    response::error::ResponseError,
    transport::TransportError,
};

/// APDU Core error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error from response processing
    #[error(transparent)]
    Response(#[from] ResponseError),

    /// Error from command processor
    #[error(transparent)]
    Processor(#[from] ProcessorError),

    /// Error from transport layer
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// Error from secure channel protocol
    #[error(transparent)]
    SecureProtocol(#[from] SecureProtocolError),

    /// Other
    #[error("Other error: {0}")]
    Other(&'static str),
}
