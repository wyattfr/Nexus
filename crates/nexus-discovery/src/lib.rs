use std::collections::HashMap;

use chrono::Utc;
use nexus_core::protocol::{NodeState, PeerInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PeerRegistry {
    peers: HashMap<String, PeerInfo>,
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_peer(&mut self, peer: PeerInfo) {
        self.peers.insert(peer.node_id.clone(), peer);
    }

    pub fn remove_peer(&mut self, node_id: &str) {
        self.peers.remove(node_id);
    }

    pub fn list_peers(&self) -> Vec<PeerInfo> {
        self.peers.values().cloned().collect()
    }

    pub fn get_peer(&self, node_id: &str) -> Option<PeerInfo> {
        self.peers.get(node_id).cloned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalNodeDirectory {
    nodes: HashMap<String, NodeState>,
}

impl LocalNodeDirectory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, node: NodeState) {
        self.nodes.insert(node.node_id.clone(), node);
    }

    pub fn list(&self) -> Vec<NodeState> {
        self.nodes.values().cloned().collect()
    }

    pub fn find_by_name(&self, name: &str) -> Option<NodeState> {
        self.nodes.values().find(|node| node.name == name).cloned()
    }

    pub fn last_seen(&self) -> String {
        Utc::now().to_rfc3339()
    }
}
