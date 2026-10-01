# ADR-0004: Secure session cryptography

Status: Accepted

NEXUS session keys are derived from fresh ephemeral X25519 key agreement,
transcript-bound with SHA-256 and HKDF. Application payloads use
ChaCha20-Poly1305 with a fresh random nonce for every frame. Session metadata
belongs in associated data so changing the session, peer, or sequence cannot
silently alter protocol interpretation.

Long-term Ed25519 identity keys authenticate the handshake and identify nodes;
they are not session encryption keys. The CLI text, RPC, and acknowledgement
paths use this session construction. The obsolete long-term-key-derived helper
was removed rather than preserved as a misleading compatibility API.
