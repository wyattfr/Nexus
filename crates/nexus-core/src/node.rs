use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeMeta {
    pub name: String,
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub public_key: String,
    pub private_key: Option<String>,
    pub created_at: String,
}
