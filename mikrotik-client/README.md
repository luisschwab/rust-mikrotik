# mikrotik-client

<p>
    <a href="https://crates.io/crates/mikrotik-client"><img src="https://img.shields.io/crates/v/mikrotik-client.svg"/></a>
    <a href="https://docs.rs/mikrotik-client"><img src="https://img.shields.io/badge/docs.rs-mikrotik--client-blue"/></a>
    <a href="https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg"/></a>
    <a href="https://github.com/luisschwab/rust-mikrotik/actions/workflows/rust.yml"><img src="https://github.com/luisschwab/rust-mikrotik/actions/workflows/rust.yml/badge.svg"></a>
</p>

Tokio client for the RouterOS binary API.

This crate uses the sans-IO `mikrotik-proto` protocol implementation with a
Tokio TCP/TLS transport. It exposes raw command execution and typed print
commands that deserialize rows into `mikrotik-types`.

## Protocol Support

Supported transports:

- API on port `8728`
- API SSL on port `8729`

The `Protocol` enum also contains service metadata for SSH, Telnet,
MAC-Telnet, FTP, HTTP, HTTPS, and WinBox so callers can share one configuration
shape across MikroTik services. Those protocols are not implemented by this
client yet.

## Usage

```rust,no_run
use mikrotik_client::builder::Builder;
use mikrotik_client::builder::Protocol;
use mikrotik_client::client::AsyncClient;
use mikrotik_client::commands;
use mikrotik_client::types::target::Credentials;

# async fn example() -> mikrotik_client::error::Result<()> {
let client = AsyncClient::connect(Builder::new(
    "10.21.21.1",
    Protocol::Api,
    Credentials {
        username: "admin".to_owned(),
        password: Some("password".to_owned()),
    },
))
.await?;

let identity = client.call("/system/identity/print", &[]).await?;
println!("identity rows: {identity:?}");

let interfaces = client
    .print::<mikrotik_client::types::api::interface::Interface>(
        commands::PrintCommand::Interface(commands::Interface::Interface),
    )
    .await?;
println!("interfaces: {}", interfaces.len());
# Ok(())
# }
```

For an account with an empty password, pass `password: Some(String::new())`.
`None` means no password was supplied to the login configuration.

## Typed Print Commands

Typed print calls use command enums instead of stringly-typed inventory:

```rust,no_run
# use mikrotik_client::commands;
# use mikrotik_client::client::AsyncClient;
# async fn example(client: &AsyncClient) -> mikrotik_client::error::Result<()> {
let rows = client
    .print::<mikrotik_client::types::api::system::Resource>(
        commands::PrintCommand::System(commands::System::Resource),
    )
    .await?;
println!("resource rows: {}", rows.len());
# Ok(())
# }
```

Use `AsyncClient::call` when you need a command that does not yet have a typed
wrapper.

## Logging and Readiness

Connection retries are logged with an optional label from
`Builder::with_log_label`. The default retry timings are conservative for real
devices. Simnet overrides them for local QEMU port-forwarded RouterOS boots.

## Development

From the workspace root:

```text
cargo rbmt fmt
cargo rbmt lint
cargo rbmt test
```

The integration-heavy behavior is exercised by `mikrotik-simnet`, which runs
real RouterOS CHR images under QEMU.
