pub mod capability;
pub mod error;
pub mod identity;
pub mod message;
pub mod node;
pub mod protocol;

pub const NEXUS_VERSION: &str = "NEXUS v0.1";

pub use capability::Capability;
pub use error::NexusError;
pub use identity::IdentityRecord;
pub use message::{MessageType, NexusMessage};
pub use node::NodeMeta;
pub use protocol::{HandshakeState, NodeState, PeerInfo, SecureFrame, SessionState};
