use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HandshakeState {
    New,
    Connecting,
    Handshaking,
    Authenticating,
    KeyEstablishment,
    Established,
    Closing,
    Closed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionState {
    pub session_id: String,
    pub local_node_id: String,
    pub peer_node_id: String,
    pub state: HandshakeState,
    pub send_sequence: u64,
    pub receive_sequence: u64,
    pub established_at: String,
    pub protocol_version: String,
    pub forward_secrecy: bool,
    pub replay_protection: bool,
}

impl SessionState {
    pub fn accept_sequence(&mut self, sequence: u64) -> Result<(), &'static str> {
        if sequence != self.receive_sequence {
            return Err(if sequence < self.receive_sequence {
                "REPLAY_DETECTED"
            } else {
                "INVALID_SEQUENCE"
            });
        }
        self.receive_sequence += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{HandshakeState, SessionState};

    fn session() -> SessionState {
        SessionState {
            session_id: "session-1".to_string(),
            local_node_id: "nxid:alice".to_string(),
            peer_node_id: "nxid:bob".to_string(),
            state: HandshakeState::Established,
            send_sequence: 0,
            receive_sequence: 0,
            established_at: "now".to_string(),
            protocol_version: "NEXUS v0.2".to_string(),
            forward_secrecy: true,
            replay_protection: true,
        }
    }

    #[test]
    fn session_rejects_replayed_and_future_sequences() {
        let mut state = session();
        assert!(state.accept_sequence(0).is_ok());
        assert_eq!(state.accept_sequence(0), Err("REPLAY_DETECTED"));
        assert_eq!(state.accept_sequence(2), Err("INVALID_SEQUENCE"));
        assert!(state.accept_sequence(1).is_ok());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecureFrame {
    pub version: String,
    pub session_id: String,
    pub sender_node_id: String,
    pub recipient_node_id: String,
    pub sequence: u64,
    pub message_id: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerInfo {
    pub name: String,
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeState {
    pub name: String,
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub public_key: String,
    pub identity_path: Option<String>,
}
