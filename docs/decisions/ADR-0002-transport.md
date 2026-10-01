# ADR-0002: Transport

Status: Accepted

The initial transport is local TCP with explicit framing and a clear wire protocol. This keeps the implementation easy to reason about and lets the protocol laboratory test real multi-process networking before adding more advanced routing or NAT mechanics.
