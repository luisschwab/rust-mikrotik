# mikrotik-fleet

[![crates.io][crates-io-badge]][crates-io-url]
[![docs.rs][docs-rs-badge]][docs-rs-url]
[![license-mit-apache][license-badge]][license-url]

[crates-io-badge]: https://img.shields.io/crates/v/mikrotik-fleet.svg
[crates-io-url]: https://crates.io/crates/mikrotik-fleet
[docs-rs-badge]: https://img.shields.io/badge/docs.rs-mikrotik--fleet-blue
[docs-rs-url]: https://docs.rs/mikrotik-fleet
[license-badge]: https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg
[license-url]: https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT

Self-contained `RouterOS` v7 fleet management application built as one Rust
binary with an Axum backend and Dioxus frontend.

Implemented capabilities include persistent fleet inventory and topology,
fixed Admin/Operator/Viewer RBAC, Argon2id passwords, server-side sessions,
confirmed TOTP enrollment and single-use recovery codes, CSRF protection,
login throttling, encrypted credential profiles, durable alerts, SMTP delivery
with a retrying outbox, encrypted scheduled/on-demand `/export` archives,
retention, an immutable audit trail, health probes, and bounded-cardinality
Prometheus metrics. The exporter includes fleet/application gauges plus
per-device CPU, memory, storage, uptime, and per-interface state, traffic,
error, and drop counters.

```text
mkdir -p mikrotik-fleet/data
cp mikrotik-fleet/mikrotik-fleet.toml.example mikrotik-fleet/data/mikrotik-fleet.toml
openssl rand -out mikrotik-fleet/data/mikrotik-fleet.key -hex 32
chmod 600 mikrotik-fleet/data/mikrotik-fleet.key
just fleet mikrotik-fleet/data/mikrotik-fleet.toml
```

The configuration path is mandatory and must be the sole command-line
argument. The process does not read application settings from environment
variables. Relative database, key, and archive paths are resolved against the
configuration file's directory.
The master key is required to decrypt credentials, MFA secrets, and backup
archives. Back it up separately from the data directory and restrict its file
permissions. Put the service behind HTTPS and set `server.secure-cookies =
true` in production. The database uses WAL mode, foreign keys, busy timeouts,
and an explicit schema version; the binary refuses databases created by a newer
schema version.

Backups use the configured read-only SSH identity to execute `/export
show-sensitive`, with an optional explicit password fallback when a router
rejects public-key authentication. Archive bytes are AES-256-GCM encrypted before their atomic
rename into the archive tree. Only authenticated users can download an archive;
the server decrypts it at that boundary and returns `Cache-Control: no-store`.

The SMTP and initial crawler credentials are deliberately supplied in the TOML
file because configuration is file-only. Restrict that file to the service
account. Credential profiles created in the UI are encrypted at rest with the
external master key and never returned in list views.

Authenticated JSON read APIs are available under `/api/v1` for inventory,
device detail, topology, alerts, and backup metadata. Operational mutations use
the browser session, fixed RBAC policy, and session-bound CSRF proof.

## Dixous RSX Formatting

To format `rsx!` code, run `dx fmt`.

## Source layout

- `backend/application`: process lifecycle and crawler workers
- `backend/auth`: authentication primitives
- `backend/backups`: SSH execution, encryption, scheduling, and retention
- `backend/config`: TOML models, loading, and validation
- `backend/domain`: backend domain types and policies
- `backend/http`: Axum routing, handlers, and request state
- `backend/metrics`: Prometheus projections
- `backend/notifications`: SMTP transport and durable outbox worker
- `backend/persistence`: storage adapters and schema
- `backend/secrets`: versioned authenticated-encryption envelopes
- `frontend/components`: reusable Dioxus components
- `frontend/document`: shared Dioxus document and application shell
- `frontend/pages`: complete Dioxus pages and page models
- `frontend/style`: embedded presentation assets

Frontend modules depend only on presentation data. Axum handlers are the
boundary that translates backend results into frontend page models.
