use std::fs;
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use nexus_core::message::{MessageType, NexusMessage};
use nexus_crypto::{
    decrypt_aead, derive_ephemeral_session_key, derive_session_id, encrypt_aead,
    generate_ephemeral_keypair, generate_identity, sign_payload, verify_signature,
};
use nexus_services::{RpcRequest, ServiceRegistry};
use nexus_transport::{bind_listener, connect, read_message, write_message};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdentityState {
    name: String,
    node_id: String,
    public_key: String,
    private_key: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ConnectionState {
    from: String,
    to: String,
    connected: bool,
    last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct InboxState {
    messages: Vec<String>,
}

struct SecureSession {
    key: [u8; 32],
    session_id: String,
    local_node_id: String,
    remote_node_id: String,
    send_sequence: u64,
    receive_sequence: u64,
}

#[derive(Parser, Debug)]
#[command(name = "nexus", version = env!("CARGO_PKG_VERSION"), about = "NEXUS protocol laboratory")]
struct Cli {
    #[arg(global = true, short, long)]
    verbose: bool,
    #[arg(global = true, short, long)]
    trace: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init,
    Identity(IdentityArgs),
    Node(NodeArgs),
    Connect {
        from: String,
        to: String,
    },
    Disconnect {
        peer: String,
    },
    Send {
        peer: String,
        message: String,
    },
    Listen {
        node: Option<String>,
    },
    Peers {
        node: Option<String>,
    },
    Status {
        node: Option<String>,
    },
    Rpc {
        peer: String,
        service: String,
        method: String,
        payload: Vec<String>,
    },
    Chat {
        peer: String,
    },
    Demo,
    Dev(DevArgs),
    Logs,
    SelfUpdate,
    Test,
    Shell,
}

#[derive(Args, Debug)]
struct IdentityArgs {
    #[command(subcommand)]
    command: IdentityCommand,
}

#[derive(Subcommand, Debug)]
enum IdentityCommand {
    Create { name: String },
    List,
    Show { name: Option<String> },
    Rotate { name: String },
}

#[derive(Args, Debug)]
struct NodeArgs {
    #[command(subcommand)]
    command: NodeCommand,
}

#[derive(Subcommand, Debug)]
enum NodeCommand {
    Start(NodeStartArgs),
    List,
    Stop { name: String },
}

#[derive(Args, Debug)]
struct NodeStartArgs {
    #[arg(long, short = 'n')]
    name: String,
    #[arg(long, short = 'p')]
    port: u16,
}

#[derive(Args, Debug)]
struct DevArgs {
    #[command(subcommand)]
    command: DevCommand,
}

#[derive(Subcommand, Debug)]
enum DevCommand {
    Network,
    Watch,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.trace {
        println!("[TRACE] NEXUS CLI startup");
    }

    match cli.command {
        Commands::Init => run_init(),
        Commands::Identity(args) => match args.command {
            IdentityCommand::Create { name } => create_identity(&name, true),
            IdentityCommand::List => list_identity_names(),
            IdentityCommand::Show { name } => show_identity(name.as_deref()),
            IdentityCommand::Rotate { name } => rotate_identity(&name),
        },
        Commands::Node(args) => match args.command {
            NodeCommand::Start(start_args) => start_node(&start_args.name, start_args.port, false),
            NodeCommand::List => list_nodes(),
            NodeCommand::Stop { name } => stop_node(&name),
        },
        Commands::Connect { from, to } => connect_nodes(&from, &to),
        Commands::Disconnect { peer } => disconnect_peer(&peer),
        Commands::Send { peer, message } => send_message(&peer, &message),
        Commands::Listen { node } => listen_messages(node.as_deref()),
        Commands::Peers { node } => list_peers(node.as_deref()),
        Commands::Status { node } => show_status(node.as_deref()),
        Commands::Rpc {
            peer,
            service,
            method,
            payload,
        } => rpc_call(&peer, &service, &method, &payload),
        Commands::Chat { peer } => chat_with_peer(&peer),
        Commands::Demo => demo(),
        Commands::Dev(args) => match args.command {
            DevCommand::Network => start_dev_network(),
            DevCommand::Watch => dev_watch(),
        },
        Commands::Logs => show_logs(),
        Commands::SelfUpdate => self_update(),
        Commands::Test => run_tests(),
        Commands::Shell => shell(),
    }
}

fn nexus_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".nexus")
}

fn identities_dir() -> PathBuf {
    let dir = nexus_dir().join("identities");
    fs::create_dir_all(&dir).ok();
    dir
}

fn nodes_dir() -> PathBuf {
    let dir = nexus_dir().join("nodes");
    fs::create_dir_all(&dir).ok();
    dir
}

fn connections_dir() -> PathBuf {
    let dir = nexus_dir().join("connections");
    fs::create_dir_all(&dir).ok();
    dir
}

fn inbox_dir() -> PathBuf {
    let dir = nexus_dir().join("inbox");
    fs::create_dir_all(&dir).ok();
    dir
}

fn default_node_file() -> PathBuf {
    nexus_dir().join("default-node.txt")
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let data = serde_json::to_string_pretty(value)?;
    fs::create_dir_all(path.parent().unwrap())
        .with_context(|| format!("failed to create {}", path.parent().unwrap().display()))?;
    fs::write(path, data).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let data =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(serde_json::from_str(&data)?)
}

fn ensure_init() -> Result<()> {
    fs::create_dir_all(nexus_dir())?;
    fs::create_dir_all(identities_dir())?;
    fs::create_dir_all(nodes_dir())?;
    fs::create_dir_all(connections_dir())?;
    fs::create_dir_all(inbox_dir())?;
    Ok(())
}

fn create_identity(name: &str, is_default: bool) -> Result<()> {
    ensure_init()?;
    let id = generate_identity(name);
    let path = identities_dir().join(format!("{name}.json"));
    let state = IdentityState {
        name: id.name,
        node_id: id.node_id,
        public_key: id.public_key_hex,
        private_key: id.private_key_hex,
        created_at: Utc::now().to_rfc3339(),
    };
    write_json(&path, &state)?;
    if is_default && !default_node_file().exists() {
        fs::write(default_node_file(), name)?;
    }
    println!("[OK] identity created for {name}");
    println!("    node_id: {}", state.node_id);
    println!("    key_path: {}", path.display());
    Ok(())
}

fn show_identity(name: Option<&str>) -> Result<()> {
    ensure_init()?;
    let names = if let Some(value) = name {
        vec![value.to_string()]
    } else {
        let mut entries = fs::read_dir(identities_dir())?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().map(|ext| ext == "json").unwrap_or(false) {
                    Some(path.file_stem()?.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        entries.sort();
        entries
    };

    if names.is_empty() {
        println!("No identities found.");
        return Ok(());
    }

    for identity_name in names {
        let path = identities_dir().join(format!("{identity_name}.json"));
        let state: IdentityState = match load_json(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        println!("{}", identity_name);
        println!("  node_id: {}", state.node_id);
        println!("  public_key: {}", state.public_key);
        println!("  created_at: {}", state.created_at);
    }
    Ok(())
}

fn list_identity_names() -> Result<()> {
    ensure_init()?;
    let mut names = fs::read_dir(identities_dir())?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().map(|ext| ext == "json").unwrap_or(false) {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    names.sort();

    if names.is_empty() {
        println!("No identities found.");
    } else {
        println!("NEXUS identities:");
        for name in names {
            println!("  {}", name);
        }
    }
    Ok(())
}

fn rotate_identity(name: &str) -> Result<()> {
    ensure_init()?;
    let _ = fs::remove_file(identities_dir().join(format!("{name}.json")));
    create_identity(name, true)
}

fn list_nodes() -> Result<()> {
    ensure_init()?;
    let mut nodes = Vec::new();
    for entry in fs::read_dir(nodes_dir())? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|ext| ext == "json").unwrap_or(false) {
            let node: NodeInfo = load_json(&path)?;
            nodes.push(node);
        }
    }
    if nodes.is_empty() {
        println!("No nodes are currently registered.");
        return Ok(());
    }
    nodes.sort_by(|a, b| a.name.cmp(&b.name));
    println!("NEXUS nodes:");
    for node in nodes {
        println!(
            "  {}  {}  {}:{}",
            node.name, node.node_id, node.address, node.port
        );
    }
    Ok(())
}

fn stop_node(name: &str) -> Result<()> {
    let path = nodes_dir().join(format!("{name}.json"));
    if path.exists() {
        fs::remove_file(path)?;
    }
    println!("[OK] stopped node {}", name);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeInfo {
    name: String,
    node_id: String,
    address: String,
    port: u16,
    public_key: String,
    created_at: String,
}

fn start_node(name: &str, port: u16, headless: bool) -> Result<()> {
    ensure_init()?;
    let identity_path = identities_dir().join(format!("{name}.json"));
    let state: IdentityState = if identity_path.exists() {
        load_json(&identity_path)?
    } else {
        let generated = generate_identity(name);
        let new_state = IdentityState {
            name: generated.name,
            node_id: generated.node_id,
            public_key: generated.public_key_hex,
            private_key: generated.private_key_hex,
            created_at: Utc::now().to_rfc3339(),
        };
        write_json(&identity_path, &new_state)?;
        new_state
    };
    let node = NodeInfo {
        name: name.to_string(),
        node_id: state.node_id.clone(),
        address: "127.0.0.1".to_string(),
        port,
        public_key: state.public_key.clone(),
        created_at: Utc::now().to_rfc3339(),
    };
    let node_path = nodes_dir().join(format!("{name}.json"));
    write_json(&node_path, &node)?;
    if !default_node_file().exists() {
        fs::write(default_node_file(), name)?;
    }

    if headless {
        run_server_loop(&state, port)?;
        return Ok(());
    }

    println!("[OK] node started: {}", name);
    println!("    node_id: {}", node.node_id);
    println!("    listen: 127.0.0.1:{}", port);
    println!("    identity: {}", identity_path.display());
    run_server_loop(&state, port)?;
    Ok(())
}

fn run_server_loop(identity: &IdentityState, port: u16) -> Result<()> {
    let listener = bind_listener(&format!("127.0.0.1:{port}"))?;
    println!("[READY] listening on 127.0.0.1:{port}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let client_identity = identity.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_incoming_connection(client_identity, &mut stream) {
                        eprintln!("[ERR] connection error: {err}");
                    }
                });
            }
            Err(err) => {
                eprintln!("[ERR] accept error: {err}");
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Ok(())
}

fn handle_incoming_connection(identity: IdentityState, stream: &mut TcpStream) -> Result<()> {
    let mut session = accept_secure_session(&identity, stream)?;
    let (message_type, plaintext) = secure_receive(stream, &mut session)?;
    match message_type {
        MessageType::Text => {
            append_inbox_message(
                &identity.name,
                &format!("{}: {}", session.remote_node_id, plaintext),
            );
            secure_send(stream, &mut session, MessageType::Ack, "received")?;
        }
        MessageType::RpcRequest => {
            let request: RpcRequest = serde_json::from_str(&plaintext)?;
            let method = request.method.clone();
            let response = ServiceRegistry::dispatch(&method, request)?;
            secure_send(
                stream,
                &mut session,
                MessageType::RpcResponse,
                &serde_json::to_string(&response)?,
            )?;
        }
        MessageType::Disconnect => {}
        _ => return Err(anyhow!("INVALID_FRAME: unexpected application message")),
    }
    Ok(())
}

fn connect_nodes(from: &str, to: &str) -> Result<()> {
    ensure_init()?;
    let from_id = load_identity_by_name(from)?;
    let to_id = load_identity_by_name(to)?;
    let mut stream = connect(&format!("127.0.0.1:{}", get_node_port(to)?))?;
    let session = establish_secure_session(&from_id, &to_id, &mut stream)?;
    let mut session = session;
    secure_send(
        &mut stream,
        &mut session,
        MessageType::Disconnect,
        "connect-check",
    )?;
    let state = ConnectionState {
        from: from.to_string(),
        to: to.to_string(),
        connected: true,
        last_seen: Utc::now().to_rfc3339(),
    };
    let path = connections_dir().join(format!("{from}-{to}.json"));
    write_json(&path, &state)?;
    println!("[OK] connected {} -> {}", from, to);
    println!("    local identity:  {}", session.local_node_id);
    println!("    remote identity: {}", session.remote_node_id);
    println!("    session:         {}", session.session_id);
    println!("    security:        authenticated + encrypted");
    println!("    forward secrecy: YES");
    Ok(())
}

fn handshake_transcript(
    hello_payload: &str,
    node_id: &str,
    public_key: &str,
    ephemeral_key: &str,
    nonce: &str,
) -> Vec<u8> {
    format!("NEXUS v0.2|{hello_payload}|{node_id}|{public_key}|{ephemeral_key}|{nonce}")
        .into_bytes()
}

fn identity_matches(node_id: &str, public_key: &str) -> bool {
    nexus_crypto::derive_node_id(public_key) == node_id
}

fn accept_secure_session(
    identity: &IdentityState,
    stream: &mut TcpStream,
) -> Result<SecureSession> {
    let hello = read_message(stream)?;
    if hello.message_type != MessageType::Hello {
        return Err(anyhow!("AUTHENTICATION_FAILED: expected HELLO"));
    }
    let hello_data: serde_json::Value = serde_json::from_str(&hello.payload)?;
    let peer_node_id = hello_data["node_id"].as_str().unwrap_or_default();
    let peer_public_key = hello_data["public_key"].as_str().unwrap_or_default();
    let peer_ephemeral = hello_data["ephemeral_public_key"]
        .as_str()
        .unwrap_or_default();
    if !identity_matches(peer_node_id, peer_public_key) {
        return Err(anyhow!(
            "IDENTITY_MISMATCH: peer node ID does not match public key"
        ));
    }
    let peer_ephemeral_bytes: [u8; 32] = hex::decode(peer_ephemeral)?
        .try_into()
        .map_err(|_| anyhow!("KEY_EXCHANGE_FAILED: invalid ephemeral key"))?;

    let local_ephemeral = generate_ephemeral_keypair();
    let server_nonce = nexus_crypto::random_id();
    let transcript = handshake_transcript(
        &hello.payload,
        &identity.node_id,
        &identity.public_key,
        &hex::encode(local_ephemeral.public_key),
        &server_nonce,
    );
    let server_signature = sign_payload(&identity.private_key, &transcript);
    let challenge_payload = serde_json::json!({
        "node_id": identity.node_id,
        "public_key": identity.public_key,
        "ephemeral_public_key": hex::encode(local_ephemeral.public_key),
        "nonce": server_nonce,
        "signature": hex::encode(server_signature),
    });
    let challenge = NexusMessage::new(
        MessageType::Challenge,
        &identity.node_id,
        peer_node_id,
        challenge_payload.to_string(),
        0,
    );
    write_message(stream, &challenge)?;

    let proof = read_message(stream)?;
    if proof.message_type != MessageType::ChallengeResponse
        || proof.sender != peer_node_id
        || proof.recipient != identity.node_id
    {
        return Err(anyhow!("AUTHENTICATION_FAILED: invalid identity proof"));
    }
    let proof_data: serde_json::Value = serde_json::from_str(&proof.payload)?;
    let proof_signature = hex::decode(proof_data["signature"].as_str().unwrap_or_default())?;
    if !verify_signature(peer_public_key, &transcript, &proof_signature) {
        return Err(anyhow!("INVALID_SIGNATURE: peer proof rejected"));
    }

    let session_id = derive_session_id(&transcript);
    let session_key =
        derive_ephemeral_session_key(&local_ephemeral, &peer_ephemeral_bytes, &transcript);
    let confirmation_signature = sign_payload(
        &identity.private_key,
        format!("NEXUS v0.2|{}|{}", session_id, hex::encode(&transcript)).as_bytes(),
    );
    let confirmation = NexusMessage::new(
        MessageType::SessionEstablished,
        &identity.node_id,
        peer_node_id,
        serde_json::json!({
            "session_id": session_id,
            "signature": hex::encode(confirmation_signature),
        })
        .to_string(),
        0,
    );
    write_message(stream, &confirmation)?;
    Ok(SecureSession {
        key: session_key,
        session_id,
        local_node_id: identity.node_id.clone(),
        remote_node_id: peer_node_id.to_string(),
        send_sequence: 0,
        receive_sequence: 0,
    })
}

fn establish_secure_session(
    local: &IdentityState,
    remote: &IdentityState,
    stream: &mut TcpStream,
) -> Result<SecureSession> {
    let local_ephemeral = generate_ephemeral_keypair();
    let hello_payload = serde_json::json!({
        "node_id": local.node_id,
        "public_key": local.public_key,
        "ephemeral_public_key": hex::encode(local_ephemeral.public_key),
        "nonce": nexus_crypto::random_id(),
    });
    let hello = NexusMessage::new(
        MessageType::Hello,
        &local.node_id,
        &remote.node_id,
        hello_payload.to_string(),
        0,
    );
    write_message(stream, &hello)?;

    let challenge = read_message(stream)?;
    if challenge.message_type != MessageType::Challenge {
        return Err(anyhow!("AUTHENTICATION_FAILED: expected challenge"));
    }
    let challenge_data: serde_json::Value = serde_json::from_str(&challenge.payload)?;
    let remote_node_id = challenge_data["node_id"].as_str().unwrap_or_default();
    let remote_public_key = challenge_data["public_key"].as_str().unwrap_or_default();
    if remote_node_id != remote.node_id || remote_public_key != remote.public_key {
        return Err(anyhow!(
            "IDENTITY_MISMATCH: expected {}, received {}",
            remote.node_id,
            remote_node_id
        ));
    }
    if !identity_matches(remote_node_id, remote_public_key) {
        return Err(anyhow!("IDENTITY_MISMATCH: remote identity is invalid"));
    }
    let remote_ephemeral = challenge_data["ephemeral_public_key"]
        .as_str()
        .unwrap_or_default();
    let remote_ephemeral_bytes: [u8; 32] = hex::decode(remote_ephemeral)?
        .try_into()
        .map_err(|_| anyhow!("KEY_EXCHANGE_FAILED: invalid ephemeral key"))?;
    let transcript = handshake_transcript(
        &hello.payload,
        remote_node_id,
        remote_public_key,
        remote_ephemeral,
        challenge_data["nonce"].as_str().unwrap_or_default(),
    );
    let server_signature = hex::decode(challenge_data["signature"].as_str().unwrap_or_default())?;
    if !verify_signature(remote_public_key, &transcript, &server_signature) {
        return Err(anyhow!("INVALID_SIGNATURE: remote challenge rejected"));
    }
    let proof = NexusMessage::new(
        MessageType::ChallengeResponse,
        &local.node_id,
        &remote.node_id,
        serde_json::json!({"signature": hex::encode(sign_payload(&local.private_key, &transcript))})
            .to_string(),
        0,
    );
    write_message(stream, &proof)?;
    let established = read_message(stream)?;
    if established.message_type != MessageType::SessionEstablished {
        return Err(anyhow!(
            "AUTHENTICATION_FAILED: session was not established"
        ));
    }
    let session_id = derive_session_id(&transcript);
    let established_data: serde_json::Value = serde_json::from_str(&established.payload)?;
    if established_data["session_id"].as_str() != Some(session_id.as_str()) {
        return Err(anyhow!("INVALID_SESSION: session binding mismatch"));
    }
    let confirmation_signature =
        hex::decode(established_data["signature"].as_str().unwrap_or_default())?;
    if !verify_signature(
        remote_public_key,
        format!("NEXUS v0.2|{}|{}", session_id, hex::encode(&transcript)).as_bytes(),
        &confirmation_signature,
    ) {
        return Err(anyhow!("INVALID_SIGNATURE: session confirmation rejected"));
    }
    let session_key =
        derive_ephemeral_session_key(&local_ephemeral, &remote_ephemeral_bytes, &transcript);
    Ok(SecureSession {
        key: session_key,
        session_id,
        local_node_id: local.node_id.clone(),
        remote_node_id: remote.node_id.clone(),
        send_sequence: 0,
        receive_sequence: 0,
    })
}

fn frame_associated_data(message: &NexusMessage, session_id: &str) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}|{:?}",
        session_id,
        message.sender,
        message.recipient,
        message.sequence,
        message.message_id,
        message.message_type
    )
    .into_bytes()
}

fn secure_send(
    stream: &mut TcpStream,
    session: &mut SecureSession,
    message_type: MessageType,
    plaintext: &str,
) -> Result<()> {
    let mut frame = NexusMessage::new(
        message_type,
        &session.local_node_id,
        &session.remote_node_id,
        "",
        session.send_sequence,
    );
    let associated_data = frame_associated_data(&frame, &session.session_id);
    let encrypted = encrypt_aead(&session.key, plaintext.as_bytes(), &associated_data);
    frame.payload = serde_json::json!({
        "session_id": session.session_id,
        "nonce": hex::encode(encrypted.nonce),
        "ciphertext": hex::encode(encrypted.ciphertext),
    })
    .to_string();
    write_message(stream, &frame)?;
    session.send_sequence += 1;
    Ok(())
}

fn secure_receive(
    stream: &mut TcpStream,
    session: &mut SecureSession,
) -> Result<(MessageType, String)> {
    let frame = read_message(stream)?;
    if frame.sender != session.remote_node_id || frame.recipient != session.local_node_id {
        return Err(anyhow!("IDENTITY_MISMATCH: frame peer binding failed"));
    }
    if frame.sequence != session.receive_sequence {
        return Err(anyhow!(
            "{}: expected {}, received {}",
            if frame.sequence < session.receive_sequence {
                "REPLAY_DETECTED"
            } else {
                "INVALID_SEQUENCE"
            },
            session.receive_sequence,
            frame.sequence
        ));
    }
    let envelope: serde_json::Value = serde_json::from_str(&frame.payload)?;
    if envelope["session_id"].as_str() != Some(session.session_id.as_str()) {
        return Err(anyhow!("INVALID_SESSION: frame session binding failed"));
    }
    let nonce: [u8; 12] = hex::decode(envelope["nonce"].as_str().unwrap_or_default())?
        .try_into()
        .map_err(|_| anyhow!("INVALID_FRAME: nonce length"))?;
    let ciphertext = hex::decode(envelope["ciphertext"].as_str().unwrap_or_default())?;
    let associated_data = frame_associated_data(&frame, &session.session_id);
    let plaintext = decrypt_aead(&session.key, &nonce, &ciphertext, &associated_data)
        .map_err(|err| anyhow!("INVALID_FRAME: {err}"))?;
    session.receive_sequence += 1;
    Ok((
        frame.message_type,
        String::from_utf8(plaintext).context("INVALID_FRAME: payload is not UTF-8")?,
    ))
}

fn disconnect_peer(peer: &str) -> Result<()> {
    let connection = connections_dir().join(format!("{}-{}.json", current_default_node()?, peer));
    if connection.exists() {
        fs::remove_file(connection)?;
    }
    println!("[OK] disconnected from {}", peer);
    Ok(())
}

fn send_message(peer: &str, message: &str) -> Result<()> {
    ensure_init()?;
    let sender_name = current_default_node()?;
    let sender = load_identity_by_name(&sender_name)?;
    let recipient = load_identity_by_name(peer)?;
    let mut stream = connect(&format!("127.0.0.1:{}", get_node_port(peer)?))?;
    let payload = serde_json::json!({ "text": message }).to_string();
    let mut session = establish_secure_session(&sender, &recipient, &mut stream)?;
    secure_send(&mut stream, &mut session, MessageType::Text, &payload)?;
    let (ack_type, ack) = secure_receive(&mut stream, &mut session)?;
    println!("[OK] sent to {}: {}", peer, message);
    if ack_type == MessageType::Ack {
        println!("    encrypted ack: {}", ack);
    }
    Ok(())
}

fn listen_messages(node: Option<&str>) -> Result<()> {
    let default_name = current_default_node().unwrap_or_else(|_| "alice".to_string());
    let used = node.unwrap_or(&default_name);
    let path = inbox_dir().join(format!("{used}.json"));
    if !path.exists() {
        println!("No messages queued for {used}.");
        return Ok(());
    }
    let state: InboxState = load_json(&path).unwrap_or_default();
    if state.messages.is_empty() {
        println!("No messages queued for {used}.");
        return Ok(());
    }
    println!("Messages for {}:", used);
    for item in state.messages {
        println!("  {}", item);
    }
    Ok(())
}

fn list_peers(node: Option<&str>) -> Result<()> {
    let default_name = current_default_node().unwrap_or_else(|_| "alice".to_string());
    let used = node.unwrap_or(&default_name);
    let mut peers = Vec::new();
    for entry in fs::read_dir(connections_dir())? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&format!("{used}-")) && name.ends_with(".json") {
            let state: ConnectionState = load_json(&entry.path())?;
            peers.push(state.to);
        }
    }
    if peers.is_empty() {
        println!("No peers for {}.", used);
        return Ok(());
    }
    println!("Known peers for {}:", used);
    for peer in peers {
        println!("  {}", peer);
    }
    Ok(())
}

fn show_status(node: Option<&str>) -> Result<()> {
    let default_name = current_default_node().unwrap_or_else(|_| "alice".to_string());
    let name = node.unwrap_or(&default_name);
    let identity = load_identity_by_name(name)?;
    let port = get_node_port(name).unwrap_or(7000);
    println!("NEXUS status");
    println!("  CLI version: {}", env!("CARGO_PKG_VERSION"));
    println!("  Protocol: 0.1");
    println!("  node: {}", name);
    println!("  node_id: {}", identity.node_id);
    println!("  port: {}", port);
    println!("  public_key: {}", identity.public_key);
    Ok(())
}

fn show_logs() -> Result<()> {
    println!("NEXUS logs");
    println!("  No persistent log file is configured in this development build.");
    println!("  Run node processes in their own terminal to observe protocol events.");
    Ok(())
}

fn self_update() -> Result<()> {
    Err(anyhow!(
        "self-update is not implemented yet; reinstall with `cargo install --path crates/nexus-cli --force`"
    ))
}

fn rpc_call(peer: &str, service: &str, method: &str, payload: &[String]) -> Result<()> {
    ensure_init()?;
    let sender_name = current_default_node()?;
    let sender = load_identity_by_name(&sender_name)?;
    let recipient = load_identity_by_name(peer)?;
    let mut stream = connect(&format!("127.0.0.1:{}", get_node_port(peer)?))?;
    let mut session = establish_secure_session(&sender, &recipient, &mut stream)?;
    let body = serde_json::json!({
        "service": service,
        "method": method,
        "params": payload,
    });
    secure_send(
        &mut stream,
        &mut session,
        MessageType::RpcRequest,
        &body.to_string(),
    )?;
    let (response_type, response_payload) = secure_receive(&mut stream, &mut session)?;
    if response_type != MessageType::RpcResponse {
        return Err(anyhow!("unexpected response from {peer}"));
    }
    let rpc: serde_json::Value = serde_json::from_str(&response_payload)?;
    println!("[OK] RPC {}:{} => {}", peer, service, rpc["data"]);
    Ok(())
}

fn chat_with_peer(peer: &str) -> Result<()> {
    println!("Chat session with {}. Type /quit to exit.", peer);
    loop {
        let mut input = String::new();
        print!("{}> ", peer);
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed == "/quit" || trimmed == "/exit" {
            println!("Leaving chat.");
            return Ok(());
        }
        if trimmed.is_empty() {
            continue;
        }
        send_message(peer, trimmed)?;
    }
}

fn run_tests() -> Result<()> {
    println!("Running NEXUS test suite through cargo test");
    let status = Command::new("cargo").args(["test"]).status()?;
    if !status.success() {
        return Err(anyhow!("cargo test failed"));
    }
    Ok(())
}

fn shell() -> Result<()> {
    println!("NEXUS interactive shell. Type 'help' or 'exit'.");
    loop {
        let mut buf = String::new();
        print!("nexus> ");
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut buf)?;
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        if line == "exit" || line == "quit" {
            println!("Goodbye.");
            return Ok(());
        }
        if line == "help" {
            println!("Commands: identity create <name>, node start <name> <port>, connect <from> <to>, send <peer> <message>, peers, status, rpc <peer> <service> <method> <args...>, exit");
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[0] == "identity" && parts[1] == "create" {
            create_identity(parts[2], true)?;
        } else if parts.len() >= 3 && parts[0] == "node" && parts[1] == "start" {
            let port = parts[3].parse::<u16>()?;
            start_node(parts[2], port, false)?;
        } else {
            println!("Unknown command. Type 'help'.");
        }
    }
}

fn run_init() -> Result<()> {
    ensure_init()?;
    println!("[OK] NEXUS initialized at {}", nexus_dir().display());
    Ok(())
}

fn demo() -> Result<()> {
    println!("========================================");
    println!("        NEXUS PROTOCOL DEMO");
    println!("========================================");
    for name in ["alice", "bob", "carol"] {
        let path = identities_dir().join(format!("{name}.json"));
        if !path.exists() {
            create_identity(name, true)?;
        }
    }
    fs::write(default_node_file(), "alice")?;

    let echo = Command::new(std::env::current_exe()?)
        .args(["node", "start", "--name", "alice", "--port", "7001"])
        .spawn()?;
    let _ = echo;
    let bob = Command::new(std::env::current_exe()?)
        .args(["node", "start", "--name", "bob", "--port", "7002"])
        .spawn()?;
    let _ = bob;
    let carol = Command::new(std::env::current_exe()?)
        .args(["node", "start", "--name", "carol", "--port", "7003"])
        .spawn()?;
    let _ = carol;

    thread::sleep(Duration::from_secs(1));
    connect_nodes("alice", "bob")?;
    connect_nodes("bob", "carol")?;
    send_message("bob", "hello from alice")?;
    send_message("carol", "hello from bob")?;
    rpc_call("bob", "echo", "echo", &["NEXUS".to_string()])?;
    println!("========================================");
    println!("DEMO COMPLETE");
    println!("========================================");
    Ok(())
}

fn start_dev_network() -> Result<()> {
    println!("NEXUS DEVELOPMENT NETWORK");
    for (name, _port) in [
        ("alice", 7001),
        ("bob", 7002),
        ("carol", 7003),
        ("dave", 7004),
    ] {
        if !identities_dir().join(format!("{name}.json")).exists() {
            create_identity(name, true)?;
        }
    }
    for (name, port) in [
        ("alice", 7001),
        ("bob", 7002),
        ("carol", 7003),
        ("dave", 7004),
    ] {
        println!("[START] {name} on 127.0.0.1:{port}");
        let _ = Command::new(std::env::current_exe()?)
            .args(["node", "start", "--name", name, "--port", &port.to_string()])
            .spawn()?;
    }
    thread::sleep(Duration::from_secs(1));
    println!("Commands:");
    println!("  connect alice bob");
    println!("  send bob hello");
    println!("  peers");
    println!("  status");
    println!("  stop");
    Ok(())
}

fn dev_watch() -> Result<()> {
    println!("Watching for rebuilds.");
    let status = Command::new("cargo")
        .args(["watch", "-x", "run --bin nexus -- demo"])
        .status()?;
    if !status.success() {
        return Err(anyhow!("cargo watch failed"));
    }
    Ok(())
}

fn current_default_node() -> Result<String> {
    let path = default_node_file();
    if !path.exists() {
        let names = fs::read_dir(identities_dir())?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().map(|ext| ext == "json").unwrap_or(false) {
                    Some(path.file_stem()?.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if names.is_empty() {
            return Err(anyhow!("no default node configured"));
        }
        let first = names.into_iter().next().unwrap();
        fs::write(&path, &first)?;
        return Ok(first);
    }
    Ok(fs::read_to_string(path)?.trim().to_string())
}

fn load_identity_by_name(name: &str) -> Result<IdentityState> {
    let path = identities_dir().join(format!("{name}.json"));
    if !path.exists() {
        return Err(anyhow!("identity '{name}' does not exist"));
    }
    load_json::<IdentityState>(&path)
}

fn get_node_port(name: &str) -> Result<u16> {
    let path = nodes_dir().join(format!("{name}.json"));
    if !path.exists() {
        return Err(anyhow!("node '{name}' is not running"));
    }
    let record: NodeInfo = load_json(&path)?;
    Ok(record.port)
}

fn append_inbox_message(node: &str, message: &str) {
    let path = inbox_dir().join(format!("{node}.json"));
    let mut state: InboxState = if path.exists() {
        load_json::<InboxState>(&path).unwrap_or_default()
    } else {
        InboxState::default()
    };
    state.messages.push(message.to_string());
    let _ = write_json(&path, &state);
}
