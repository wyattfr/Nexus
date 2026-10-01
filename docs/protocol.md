# Protocol notes

The current wire format is a versioned message envelope with cryptographic
node IDs, session-bound sequence numbers, and an encrypted application
payload. Human-readable aliases are not sent as protocol identities.

```text
NexusMessage {
  version: "NEXUS v0.1",
  message_type: Hello | Challenge | ChallengeResponse | SessionEstablished | Text | RpcRequest | RpcResponse | Disconnect | Ack,
  message_id: String,
  sender: nxid,
  recipient: nxid,
  timestamp: String,
  sequence: u64,
  payload: encrypted frame envelope,
  signature: Option<String>,
}
```

The handshake authenticates both Ed25519 identity keys, exchanges fresh
ephemeral X25519 public keys, binds them to a transcript, and derives a
transcript-bound HKDF session key. Application frames use ChaCha20-Poly1305
with fresh random nonces. Session ID, sender, recipient, message type, and
sequence are authenticated as associated data. Replayed and out-of-order
frames are rejected.

Messages are serialized with bincode and sent with a 4-byte length prefix over
TCP. TCP endpoints are routing hints only; they do not establish identity.
