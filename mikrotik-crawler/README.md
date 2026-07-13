# mikrotik-crawler

Long-running read-only RouterOS discovery and snapshot collection.

This crate keeps a target registry in memory, refreshes `DeviceSnapshot` values
with Tokio background tasks, and emits snapshot events for observers.

## Development

From the workspace root:

```text
cargo rbmt fmt
cargo rbmt lint
cargo rbmt test
cargo rbmt docs
```
