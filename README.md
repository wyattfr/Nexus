# NEXUS

NEXUS is a minimal, working P2P protocol laboratory built for rapid local experimentation. It has persistent cryptographic identities, real local nodes, authenticated handshakes, encrypted messages, explicit framing, and an RPC echo service.

## First run

### Windows PowerShell

```powershell
git clone <repository>
cd nexus
.\scripts\install.ps1
nexus demo
```

The installer builds the existing Rust CLI, installs `nexus.exe` into the
user's Cargo binary directory, adds that directory to the user PATH when
needed, and verifies `nexus --version` and `nexus --help`. Reopen terminals
created before installation if they do not see the updated PATH.

### Linux/macOS

```bash
git clone <repository>
cd nexus
./scripts/install.sh
nexus demo
```

## Installed CLI

After installation, use the normal command directly:

```text
nexus --help
nexus --version
nexus identity create alice
nexus identity list
nexus node start --name alice --port 7001
nexus node start --name bob --port 7002
nexus peers
nexus connect alice bob
nexus send bob "hello"
nexus listen bob
nexus rpc bob echo echo "hello world"
nexus status
nexus logs
```

`node start` runs continuously, so start each node in a separate terminal.
Stop a node process with Ctrl+C. The node registry can also be removed with:

```text
nexus node stop alice
```

The current CLI uses positional node arguments for `listen`, `peers`, and
`status`, for example `nexus listen bob`.

## Development commands

The Cargo form remains supported and invokes the same binary:

```bash
cargo build
cargo test
cargo run --bin nexus -- demo
```

Convenience commands:

```text
make build
make test
make install
make uninstall
make demo
```

On Windows, where Make may not be installed:

```powershell
.\scripts\test.ps1
.\scripts\demo.ps1
.\scripts\dev.ps1
```

For process-level hot reloading, install `cargo-watch` once:

```text
cargo install cargo-watch
nexus dev watch
```

This rebuilds and reruns the development demo when Rust sources change.

## Architecture

```text
Applications
  ├── Chat
  ├── RPC / APIs
  ├── Pub/Sub
  └── Custom Services
        │
        ▼
Service Layer
        │
        ▼
Capability / Authorization Layer
        │
        ▼
Identity Layer
        │
        ▼
Secure Session Layer
        │
        ▼
P2P Transport
        │
        ▼
Discovery / Routing
        │
        ▼
Underlying Network
```

The CLI binary is the single implementation in
[`crates/nexus-cli`](./crates/nexus-cli). It uses:

- `nexus-core` for messages, identities, capabilities, and protocol types
- `nexus-crypto` for Ed25519 signatures and ChaCha20-Poly1305 payload encryption
- `nexus-transport` for framed TCP messages
- `nexus-discovery` for local peer and node registries
- `nexus-services` for RPC and chat service abstractions

## Versioning

The CLI package version is `0.1.0`. The wire protocol reports `NEXUS v0.1`.
The installed binary and `cargo run --bin nexus -- ...` use the same source
and package version.

## Security note

This is an initial protocol laboratory, not a production-ready secure
network. It uses established primitives, but authorization, replay protection,
NAT traversal, capability delegation, and hardened session management remain
experimental or planned. See [`docs/security.md`](./docs/security.md).
