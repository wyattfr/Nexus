use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RpcRequest {
    pub method: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RpcResponse {
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
}

pub trait NexusService {
    fn name(&self) -> &str;
    fn handle(&self, request: RpcRequest) -> Result<RpcResponse>;
}

#[derive(Debug, Clone, Default)]
pub struct EchoService;

impl NexusService for EchoService {
    fn name(&self) -> &str {
        "echo"
    }

    fn handle(&self, request: RpcRequest) -> Result<RpcResponse> {
        let payload = request.params.join(" ");
        Ok(RpcResponse {
            success: true,
            data: payload,
            error: None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChatService;

impl NexusService for ChatService {
    fn name(&self) -> &str {
        "chat"
    }

    fn handle(&self, request: RpcRequest) -> Result<RpcResponse> {
        Ok(RpcResponse {
            success: true,
            data: format!("chat relay: {}", request.params.join(" ")),
            error: None,
        })
    }
}

pub struct ServiceRegistry;

impl ServiceRegistry {
    pub fn dispatch(service_name: &str, request: RpcRequest) -> Result<RpcResponse> {
        match service_name {
            "echo" => EchoService.handle(request),
            "chat" => ChatService.handle(request),
            _ => Ok(RpcResponse {
                success: false,
                data: String::new(),
                error: Some(format!("unknown service: {service_name}")),
            }),
        }
    }
}
