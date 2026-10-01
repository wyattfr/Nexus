# Architecture

NEXUS is intentionally small and modular. It is designed as a development protocol laboratory where each layer can evolve without destabilizing the rest of the protocol stack.

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

The initial milestone focuses on identity, transport, authenticated handshake, encrypted messages, local networking, and the CLI-first developer workflow.
