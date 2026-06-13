<div align="center">

<h1>Rust MikroTik</h1>

<p>
Rust crates for talking to MikroTik RouterOS, decoding RouterOS API rows, and
validating behavior against QEMU/CHR simulated networks.
</p>

<p>
<a href="https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/">
    <img src="https://img.shields.io/badge/rustc-1.85.0%2B-orange.svg?label=MSRV"/>
</a>
<a href="https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT">
    <img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg"/>
</a>
<a href="https://github.com/luisschwab/rust-mikrotik/actions/workflows/rust.yml">
    <img src="https://github.com/luisschwab/rust-mikrotik/actions/workflows/rust.yml/badge.svg">
</a>
<a href="https://github.com/luisschwab/rust-mikrotik/actions/workflows/simnet.yml">
    <img src="https://github.com/luisschwab/rust-mikrotik/actions/workflows/simnet.yml/badge.svg">
</a>
</p>

</div>

## Crates

Crates | Purpose
---|---
[`mikrotik-client`](mikrotik-client) | Tokio-based RouterOS binary API client with raw calls and typed print commands.
[`mikrotik-types`](mikrotik-types) | Versionless RouterOS API response models and reusable RouterOS primitive types.
[`mikrotik-simnet`](mikrotik-simnet) | Internal QEMU/CHR topology runner used to exercise the client and types against real RouterOS images.


## RouterOS API Client

`mikrotik-client` supports the RouterOS binary API transports:

- API on port `8728`
- API SSL on port `8729`

Other MikroTik management protocols such as SSH, Telnet, MAC-Telnet, HTTP,
HTTPS, FTP, and WinBox are represented in the protocol enum for service
metadata, but they are not implemented as client transports yet.

```rust,no_run
use mikrotik_client::builder::Builder;
use mikrotik_client::builder::Protocol;
use mikrotik_client::client::AsyncClient;
use mikrotik_client::commands;
use mikrotik_client::types::target::Credentials;

# async fn example() -> mikrotik_client::error::Result<()> {
let client = AsyncClient::connect(Builder::new(
    "192.0.2.1",
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

## Simulated Networks

`mikrotik-simnet` runs deterministic RouterOS topologies using MikroTik CHR
images under QEMU. It downloads or reuses CHR images, starts one QEMU process
per router, waits for API readiness with a rolling VM window, applies bootstrap
commands, runs checks, and writes CSV/Mermaid/log artifacts under
`mikrotik-simnet/.chr-cache/runs`.

Run a one-shot scenario from the workspace root:

```text
cargo rbmt run run -p mikrotik-simnet -- run --non-interactive three-router-bgp.toml
```

Render a topology diagram without starting QEMU:

```text
cargo rbmt run run -p mikrotik-simnet -- mermaid three-router-bgp.toml
```

See [mikrotik-simnet/README.md](mikrotik-simnet/README.md) for host
requirements, topology format, bundled scenarios, and debugging notes.

## Development

Use the repo wrapper commands rather than invoking raw Cargo tasks directly:

```text
cargo rbmt lock
cargo rbmt fmt
cargo rbmt lint
cargo rbmt test
```

The normal test suite covers unit tests, doctests, feature combinations, and
the no-std build for `mikrotik-types`. Full QEMU topology runs are separate
because they depend on host QEMU support and live RouterOS CHR images.

## Minimum Supported Rust Version

This set of libraries should compile with any combination of features on Rust 1.85.0.

To build with the MSRV toolchain, copy `Cargo-minimal.lock` to `Cargo.lock`.

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
