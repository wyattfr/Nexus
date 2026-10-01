# Security model

## Threat model

The initial implementation recognizes the following credible threats:

- passive observers
- active MITM attackers
- replay attacks
- malicious peer behavior
- spoofed identities
- packet tampering
- compromised local state

## State

- IMPLEMENTED: persistent Ed25519 identities, deterministic node IDs, signed messages, local TCP transport, handshake verification
- IMPLEMENTED: the CLI handshake now authenticates both Ed25519 identities, verifies the expected Node ID, binds fresh X25519 ephemeral keys to a transcript, and establishes an AEAD session
- IMPLEMENTED: CLI application frames use transcript-derived session keys, random per-frame nonces, authenticated session metadata, and strict receive sequencing
- EXPERIMENTAL: session state is currently scoped to each request connection rather than maintained as a long-lived multiplexed stream
- PLANNED: NAT traversal, relay networks, public discovery, DHT, capability delegation, distributed service routing

## Important note

Encryption alone does not make a protocol secure. The current CLI path now uses
the secure session APIs for handshake, text, RPC, and encrypted acknowledgements,
but this remains a working laboratory rather than a production authorization
or consensus system.
