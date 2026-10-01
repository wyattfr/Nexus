# ADR-0001: Identity model

Status: Accepted

We use a stable Ed25519 keypair per node. Node IDs are based on public-key-derived hashes and are persisted under `~/.nexus/identities/`. The goal is clear, stable identity without premature complexity.
