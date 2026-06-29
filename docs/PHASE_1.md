# Phase 1: Audit-only MVP

This phase implements the foundational audit capabilities:
- `mirage-providers`: Implemented `IdentityProvider` reading for Hostname, Machine-ID, Timezone, Locale, IPv4, IPv6, WebRTC (local interfaces + STUN simulation via ipify), DNS, and stubs for GPS, Wi-Fi, and Bluetooth.
- `mirage-protocol`: Added typed `SignalKind` and `SignalValue` payloads.
- `mirage-core`: Implemented `AuditEngine` that queries all registered providers, and a `ConsistencyEngine` with a `BasicNetworkRule` (cross-checking WebRTC against System IPv4).
- `mirage-cli`: Implemented `mirage audit` CLI command that prints out the collected signals and the consistency score breakdown.
- `mirage-gui`: Minimal Tauri + React dashboard wired to invoke the `run_audit` command via IPC and display the identity graph payload.

All Phase 1 requirements are met. Next is Phase 2 (Sandboxed projection v1).
