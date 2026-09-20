use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityRecord {
    pub name: String,
    pub node_id: String,
    pub public_key: String,
    pub private_key: String,
    pub created_at: String,
}
