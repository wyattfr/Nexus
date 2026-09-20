use thiserror::Error;

#[derive(Debug, Error)]
pub enum NexusError {
    #[error("{0}")]
    Message(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("identity not found: {0}")]
    IdentityNotFound(String),
    #[error("invalid signature for {0}")]
    InvalidSignature(String),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("peer not found: {0}")]
    PeerNotFound(String),
    #[error("unknown message type: {0}")]
    UnknownMessageType(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    #[error("config error: {0}")]
    Config(String),
}
