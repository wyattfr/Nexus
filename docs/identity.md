# Identity model

Each node generates a stable Ed25519 identity. The node ID is deterministically derived from the public key and stored under `~/.nexus/identities/`.

This creates a clean initial identity model:

- stable node ID
- persistent keypair
- deterministic derivation from public key
- simple CLI management commands
