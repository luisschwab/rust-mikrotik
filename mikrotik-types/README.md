# mikrotik-types

<p>
    <a href="https://crates.io/crates/mikrotik-types"><img src="https://img.shields.io/crates/v/mikrotik-types.svg"/></a>
    <a href="https://docs.rs/mikrotik-types"><img src="https://img.shields.io/badge/docs.rs-mikrotik--types-blue"/></a>
    <a href="https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg"/></a>
    <a href="https://github.com/luisschwab/rust-mikrotik/actions/workflows/rust.yml"><img src="https://github.com/luisschwab/rust-mikrotik/actions/workflows/rust.yml/badge.svg"></a>
</p>

Versionless data models for MikroTik RouterOS API responses.

RouterOS API replies are string maps whose fields vary by RouterOS version,
installed package, hardware platform, and local configuration. This crate
provides typed row structs and reusable primitive parsers while keeping that
variability explicit with optional fields.

## Layout

Module | Purpose
---|---
`api` | RouterOS API response rows grouped by menu namespace.
`primitives` | Reusable RouterOS scalar types such as IDs, byte sizes, durations, IP prefixes, and routing values.
`target` | Connection target and credential types shared with `mikrotik-client`.
`device` | Higher-level device snapshot models composed from API rows.
`topology` | Higher-level topology/domain models.

Endpoint structs follow the RouterOS menu path in `PascalCase`. For example:

RouterOS command | Rust type
---|---
`/interface/print` | `api::interface::Interface`
`/interface/ethernet/switch/port/print` | `api::interface::EthernetSwitchPort`
`/ip/settings/print` | `api::ip::IpSettings`
`/routing/stats/origin/print` | `api::routing::RoutingStatsOrigin`

## Usage

Rows can be deserialized directly from raw RouterOS API maps:

```rust
use mikrotik_types::Row;
use mikrotik_types::api::system::Identity;

let row = Row::from([("name".to_owned(), "lab-router".to_owned())]);
let identity: Identity = mikrotik_types::deserialize(&row)?;

assert_eq!(identity.name.as_deref(), Some("lab-router"));
# Ok::<(), serde_json::Error>(())
```

With `mikrotik-client`, typed print commands deserialize into these structs:

```rust,no_run
# use mikrotik_client::commands;
# use mikrotik_client::client::AsyncClient;
# async fn example(client: &AsyncClient) -> mikrotik_client::error::Result<()> {
let resources = client
    .print::<mikrotik_client::types::api::system::Resource>(
        commands::PrintCommand::System(commands::System::Resource),
    )
    .await?;
println!("resource rows: {}", resources.len());
# Ok(())
# }
```

## no_std

`mikrotik-types` is `no_std` with `alloc`. The workspace test task builds the
crate for a no-std target as part of normal verification.

## Coverage

The crate is intentionally versionless: one type attempts to model the stable
shape of an endpoint across RouterOS versions, with optional fields for values
that are version-, platform-, package-, or configuration-dependent.

Fixture tests cover captured live rows, and `mikrotik-simnet` exercises typed
deserialization against real CHR images across the cataloged RouterOS versions.

## Development

From the workspace root:

```text
cargo rbmt fmt
cargo rbmt lint
cargo rbmt docs
cargo rbmt test
```
