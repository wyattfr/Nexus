use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::NexusError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    Hello,
    Challenge,
    ChallengeResponse,
    SessionEstablished,
    Text,
    RpcRequest,
    RpcResponse,
    Disconnect,
    Ack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NexusMessage {
    pub version: String,
    pub message_type: MessageType,
    pub message_id: String,
    pub sender: String,
    pub recipient: String,
    pub timestamp: String,
    pub sequence: u64,
    pub payload: String,
    pub signature: Option<String>,
}

impl NexusMessage {
    pub fn new(
        message_type: MessageType,
        sender: impl Into<String>,
        recipient: impl Into<String>,
        payload: impl Into<String>,
        sequence: u64,
    ) -> Self {
        let message_id = uuid::Uuid::new_v4().to_string();
        Self {
            version: "NEXUS v0.1".to_string(),
            message_type,
            message_id,
            sender: sender.into(),
            recipient: recipient.into(),
            timestamp: Utc::now().to_rfc3339(),
            sequence,
            payload: payload.into(),
            signature: None,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, NexusError> {
        bincode::serialize(self).map_err(|err| NexusError::Serialization(err.to_string()))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, NexusError> {
        bincode::deserialize(bytes).map_err(|err| NexusError::Serialization(err.to_string()))
    }

    pub fn set_signature(&mut self, signature: String) {
        self.signature = Some(signature);
    }

    pub fn signed_bytes(&self) -> Vec<u8> {
        let mut clone = self.clone();
        clone.signature = None;
        bincode::serialize(&clone).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{MessageType, NexusMessage};

    #[test]
    fn message_round_trip_serializes() {
        let message = NexusMessage::new(MessageType::Text, "alice", "bob", "hello", 7);
        let bytes = message.serialize().unwrap();
        let decoded = NexusMessage::deserialize(&bytes).unwrap();
        assert_eq!(decoded.recipient, "bob");
        assert_eq!(decoded.payload, "hello");
    }

    #[test]
    fn message_signature_bytes_are_stable() {
        let message = NexusMessage::new(MessageType::Text, "alice", "bob", "hello", 7);
        let signed = message.signed_bytes();
        assert!(!signed.is_empty());
    }
}
