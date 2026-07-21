# Contributing

Run development commands from the workspace root. The repository uses
[`cargo-rbmt`](https://crates.io/crates/cargo-rbmt) to apply the same tasks to
the workspace crates and `just` as the concise command interface.

## Setup

Install the configured Rust toolchains, development tools, and QEMU with:

```text
just setup
```

The component recipes can also be run independently:

```text
just toolchains
just tools
just qemu-deps
```

`qemu-deps` currently uses Homebrew and is intended for macOS development
hosts.

## `cargo rbmt`

The primary workspace tasks are:

```text
cargo rbmt fmt
cargo rbmt fmt --check
cargo rbmt lint
cargo rbmt test
cargo rbmt docs
cargo rbmt docs --open
cargo rbmt lock
```

Restrict a task to one crate with `-p`:

```text
cargo rbmt -p mikrotik-fleet test
cargo rbmt -p mikrotik-graphviz lint
```

The complete test matrix can select a configured toolchain and generated
lockfile explicitly:

```text
cargo rbmt test --toolchain msrv --lockfile minimal
cargo rbmt test --toolchain stable --lockfile minimal
cargo rbmt test --toolchain stable --lockfile recent
```

## Justfile recipes

Prefer these recipes for routine work:

| Recipe | Purpose |
| --- | --- |
| `just fmt` | Format all workspace crates. |
| `just check` | Check formatting, run Clippy, and build documentation. |
| `just test` | Run the standard workspace test and feature matrix. |
| `just test-all` | Test the MSRV/stable and minimal/recent lockfile combinations. |
| `just docs` | Build documentation. |
| `just docs-open` | Build and open documentation. |
| `just lock` | Regenerate the workspace lockfiles. |
| `just audit` | Audit all generated lockfiles. |
| `just zizmor` | Audit workflow definitions with Zizmor. |
| `just pre-push` | Run the aggregate pre-push verification sequence. |
| `just pkg <package> <task>` | Run one rbmt task for one package. |
| `just run <package> <args...>` | Run one package through rbmt with application arguments. |
| `just examples <package>` | Test every example belonging to one package. |
| `just scenario <test>` | Run one ignored QEMU scenario test. |
| `just crawler-scenario` | Crawl the default QEMU scenario example. |
| `just crawl [scenario]` | Run the crawler against a scenario manifest. |
| `just one-device` | Spawn one RouterOS CHR. |
| `just two-devices` | Spawn two linked RouterOS CHRs. |

The current `pre-push` sequence references a `shellcheck` recipe that is not
defined in this Justfile, so it stops at that step until the recipe is restored
or the stale invocation is removed. The individual formatting, check, test,
Zizmor, and audit recipes remain usable directly.

Frequently used aliases are `just f`, `just c`, `just t`, `just l`, `just a`,
`just z`, and `just p`. Run `just --list` for the authoritative recipe list.

For example, start the fleet application with its mandatory configuration
path using:

```text
just fleet mikrotik-fleet/data/mikrotik-fleet.toml
```

## Before submitting

At minimum, format and verify the workspace:

```text
just fmt
just check
just test
```

Do not commit runtime databases, keys, backup archives, or other files from a
crate's ignored `data` directory.

## Code style

### Types

- Prefer using domain-specific types instead of generics (e.g.
  use `DeviceSerial`instead of `String`).

### Structure

- Keep crates modular and responsibilities atomic.
- Split large features into focused modules and submodules.
- Keep backend and frontend concerns separate.
- Put module-wide attributes in the nearest common `mod.rs` instead of
  repeating identical attributes in every child module.
- Keep shared domain types in the lowest appropriate crate. RouterOS API data
  belongs in `mikrotik-types`, collection envelopes and crawler behavior belong
  in `mikrotik-crawler`, and fleet-specific inventory and application state
  belong in `mikrotik-fleet`.
- Avoid compatibility layers for unreleased interfaces unless they are
  explicitly required.

### Imports

- Put imports at the top of the module.
- Do not use fully qualified paths such as `std::time::Duration` in function
  bodies when a normal import is appropriate.
- Prefer `core` over `std` for APIs available in `core` and supported by the
  workspace MSRV.
- Import one item per line and let `rustfmt` group and order imports.
- Use anonymous trait imports such as `use core::fmt::Write as _;` when the
  trait name itself is not referenced.

### MSRV compatibility

- The workspace MSRV is defined by `workspace.package.rust-version` in the
  root `Cargo.toml`.
- All production code, tests, examples, build scripts, and documentation
  examples must compile with the MSRV toolchain.
- Stable and nightly are additional RBMT verification toolchains; they do not
  define the minimum supported Rust version.
- Do not introduce language features, standard-library APIs, dependency
  versions, or Cargo features requiring Rust newer than the declared MSRV.
- Changes to the workspace MSRV must be deliberate and made explicitly in the
  root `Cargo.toml`.
- Run the MSRV and minimal-lockfile matrix before submitting
  compatibility-sensitive changes:

```text
cargo rbmt test --toolchain msrv --lockfile minimal
```

### Formatting and linting

- Follow the repository `rustfmt.toml`: 120-column code and comments, formatted
  code in documentation comments, item-level import granularity, and grouped
  standard-library, external-crate, and local imports.
- Run `just fmt`; do not manually fight `rustfmt`.
- Treat Clippy warnings as errors.
- Keep lint exceptions narrow and explain every exception with a `reason`.
- Put a shared lint exception on the nearest common module instead of
  duplicating it in multiple files.

### Documentation

- Document public items and non-obvious private behavior.
- Explain why a workaround or unusual implementation exists, not merely what
  it does.
- Include `# Errors`, `# Panics`, and `# Safety` sections where applicable.
- Keep documentation links valid; broken rustdoc links fail verification.
- Spell product names consistently, including `MikroTik`, `RouterOS`, and
  `RouterBOARD`.

### Types and APIs

- Prefer typed domain models over loosely structured JSON or string maps.
- Keep transport data, collected metadata, and fleet state distinct.
- Group RouterOS data by its RouterOS section.
- Preserve endpoint-local failures alongside endpoint data instead of silently
  replacing failures with empty results.
- Use enums for closed sets of states rather than magic strings.
- Prefer constructors and focused methods over repeated direct struct
  manipulation.
- Derive traits only when they are meaningful for the type.
- Keep secrets out of `Debug`, `Display`, logs, errors, and serialized public
  responses.

### Error handling

- Return structured errors with useful context.
- Include the exact RouterOS command that caused a command failure.
- Distinguish transport, timeout, authentication, permission,
  unsupported-command, decoding, and RouterOS trap failures.
- Do not silently discard errors.
- Avoid `unwrap` and `expect` outside tests unless an invariant makes failure
  impossible and the invariant is documented.

### Configuration

- Do not use environment variables for application configuration.
- Read application configuration from TOML.
- Application binaries requiring configuration must require an explicit
  configuration path.
- Define defaults as constants so omitted TOML fields are populated
  consistently.
- Resolve relative paths against the configuration file's directory.
- Persist WebUI configuration changes to both SQLite and the TOML file.

### Dependencies

- Prefer workspace dependencies where available.
- Write external dependencies using table syntax:

```toml
rand = { version = "0.10.2" }
```

- Avoid duplicate major versions or dependency aliases unless required.
- Put deliberately pinned transitive dependencies below normal dependencies
  under a `# Pinned transitive deps:` comment.
- Add a short `# blame:` comment explaining the dependency chain requiring
  each transitive pin.
- Keep dependencies current without violating the workspace MSRV.

### Frontend

- Preserve source-data casing. Never automatically capitalize device names or
  other RouterOS values.
- Write intentionally uppercase interface labels directly in RSX.
- Do not use CSS `text-transform` to supply semantic casing.
- Build repeated controls such as dropdowns, validation feedback, and
  navigation as reusable components.
- Keep page rendering separate from backend persistence and collection logic.
- Render endpoint errors explicitly when endpoint data is unavailable.
- Enforce authorization on the backend even when controls are hidden or
  disabled in the frontend.
- Do not use aria labels.

### Tests and verification

- Add tests with every behavioral change or regression fix.
- Test successful data and endpoint-local failure cases.
- Prefer realistic typed snapshot fixtures.
- Use `just` recipes for routine repository operations.
- Before submitting, run:

```text
just fmt
just check
just test
```
