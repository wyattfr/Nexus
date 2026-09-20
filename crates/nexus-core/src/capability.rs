use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    pub issuer: String,
    pub subject: String,
    pub service: String,
    pub action: String,
    pub resource: String,
    pub expiration: String,
    pub restrictions: Vec<String>,
    pub signature: String,
}
