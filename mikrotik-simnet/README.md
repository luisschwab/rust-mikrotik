# mikrotik-simnet

`mikrotik-simnet` is an internal QEMU/CHR simulation harness for running
RouterOS topologies against the `rust-mikrotik` client and type crates.

It reads a small deterministic TOML topology file, downloads or reuses MikroTik
CHR images, starts one QEMU VM per router, applies bootstrap commands over the
RouterOS API, runs checks, then leaves the scenario running until Ctrl-C.

## Prerequisites

Install the host tools used by the runner:

```text
curl
unzip
qemu-img
qemu-system-x86_64 or qemu-system-aarch64
```

On Apple Silicon, AArch64 CHR uses UEFI firmware from QEMU. A Homebrew QEMU
install normally provides:

```text
/opt/homebrew/share/qemu/edk2-aarch64-code.fd
/opt/homebrew/share/qemu/edk2-arm-vars.fd
```

The harness chooses the guest architecture from the host and RouterOS version.
When hardware acceleration is unavailable, set `allow_software_emulation = true`
in the topology to allow QEMU TCG.

## Running a Scenario

From the workspace root, build the project through the wrapper flow first:

```text
cargo rbmt test
```

Then run scenarios through the wrapper:

```text
cargo rbmt run run -p mikrotik-simnet -- run single-router.toml
cargo rbmt run run -p mikrotik-simnet -- run two-router.toml
cargo rbmt run run -p mikrotik-simnet -- run three-router-bgp.toml
cargo rbmt run run -p mikrotik-simnet -- run stress-test.toml
```

The bundled scenarios are:

- `single-router.toml`: one CHR, API bootstrap, resource check.
- `two-router.toml`: two CHRs with a point-to-point link, IPv4 and IPv6 routes.
- `three-router-bgp.toml`: three CHRs with two links and BGP route checks.
- `stress-test.toml`: one CHR for every cataloged RouterOS version,
  chained together with BGP, VLAN, DHCP, firewall, NAT, WireGuard, GRE, VRF, PPP,
  L2TP, IPsec objects, static routes, and all print command checks.

After bootstrap and checks pass, the scenario stays up for inspection. Press
Ctrl-C to stop all router processes and remove the temporary runtime socket
directory.

For CI or one-shot validation, stop the scenario as soon as checks pass:

```text
cargo rbmt run run -p mikrotik-simnet -- run --non-interactive stress-test.toml
```

Render a topology manifest as a Mermaid flowchart without starting QEMU:

```text
cargo rbmt run run -p mikrotik-simnet -- mermaid two-router.toml
```

## Topology Format

Topologies use a deliberately small TOML subset:

```toml
name = "example"
allow_software_emulation = true

[[routers]]
name = "R1"
version = "7.23.1"
memory_mib = 256
cpus = 1
bootstrap = [
  "/system/identity/set name=R1",
  "/ip/address/add address=192.0.2.1/30 interface=ether2",
]

[[routers]]
name = "R2"
version = "7.23.1"
bootstrap = [
  "/system/identity/set name=R2",
  "/ip/address/add address=192.0.2.2/30 interface=ether2",
]

[[links]]
a = "R1:ether2"
b = "R2:ether2"

[[checks]]
type = "command-rows"
router = "R1"
command = "/interface/print"
min_rows = 1
```

Supported top-level fields:

- `name`: required topology name.
- `allow_software_emulation`: optional boolean, defaults to `false`.

Supported router fields:

- `name`: required router name.
- `version`: required RouterOS version from the built-in catalog.
- `memory_mib`: optional memory size, defaults to `256`.
- `cpus`: optional vCPU count, defaults to `1`.
- `bootstrap`: optional array of RouterOS API commands.

Supported link fields:

- `a`: required endpoint in `router:etherN` form.
- `b`: required endpoint in `router:etherN` form.

Supported checks:

- `type = "command-rows"` runs a RouterOS API command and requires at least
  `min_rows` rows.
- `type = "all-print-commands"` runs all print command checks shared with
  the live-router test.

`all-print-commands` also accepts `allow_unsupported = true`. Use that when a
topology spans RouterOS versions where a missing package or platform-only
endpoint should be counted as skipped, while row decode failures still fail the
run.

## Version Catalog

The CHR image architecture is inferred automatically from the host architecture
and the image architectures available for each RouterOS version. For example,
RouterOS 7.x uses the native CHR architecture when available, while RouterOS 6.x
uses the x86_64 CHR image because that is the only RouterOS 6 CHR image.

List cataloged versions, release channels, inferred guest architecture, and
available CHR image architectures:

```text
cargo rbmt run run -p mikrotik-simnet -- list-versions
```

Run each topology explicitly:

```text
cargo rbmt run run -p mikrotik-simnet -- run single-router.toml
```

## Bootstrap Commands

Bootstrap entries are RouterOS API commands written as one string:

```text
/path/to/command key=value flag
```

The command path must start with `/`. Attributes are split on whitespace; values
with spaces are not supported by the current parser.

The harness waits until all linked interfaces declared in the manifest are
visible before bootstrap commands run. This keeps commands like
`/ip/address/add interface=ether2 ...` deterministic.

## Runtime State

Persistent CHR downloads and run artifacts live under:

```text
mikrotik-simnet/.chr-cache
```

Important subdirectories:

- `images`: cached raw CHR images.
- `runs`: per-run overlays, logs, reports, QEMU argument snapshots, and pid files.

Point-to-point QEMU stream sockets are created under `/tmp/mikrotik-simnet-<run>`
to avoid Unix socket path length limits. They are removed when the run exits.

## Debugging

Run with debug logging to print run directories and file paths:

```text
RBMT_LOG_LEVEL=debug cargo rbmt run run -p mikrotik-simnet -- run single-router.toml
```

Inspect these files first:

- `<router>.serial.log`: RouterOS serial console output.
- `<router>.qemu.log`: QEMU stderr.
- `<router>.qemu.args`: exact QEMU command line used by the harness.
- `<router>.print-commands.csv`: per-router print command report.
- `topology.csv`: per-run topology and runtime target report.
- `topology.mmd`: Mermaid topology diagram.

Common failures:

- Missing QEMU binary or firmware files: install QEMU for the relevant host.
- API timeout: inspect the serial log for RouterOS boot progress.
- Missing link interface: inspect `<router>.qemu.args` and verify the topology
  endpoint uses `etherN`.
- Bootstrap command failure: run the same RouterOS command manually against a
  CHR of the same version and check for RouterOS syntax changes.

## Verification

Use the workspace wrapper commands for repo checks:

```text
cargo rbmt lock
cargo rbmt fmt
cargo rbmt lint
cargo rbmt test
```

The normal test suite does not run every QEMU topology by default. Run the
scenario files explicitly when changing simulator boot, networking, or RouterOS
bootstrap behavior.

Optional QEMU/CHR test gates:

```text
MIKROTIK_SIMNET=1 cargo rbmt test
```

- `MIKROTIK_SIMNET=1` runs the basic single-router gated scenario.

The GitHub Actions `Simnet` workflow runs separately from normal pull request
checks because it depends on MikroTik's live download service and QEMU. Its
matrix explicitly lists each topology file to run. When a new topology should
run in CI, add it to the workflow matrix.

CI caches downloaded CHR base images from `mikrotik-simnet/.chr-cache/images`
between runs, then uploads the selected run artifacts from
`mikrotik-simnet/.chr-cache/runs`. Use those artifacts to identify the exact
RouterOS version, topology, selected guest architecture, and boot/API state
behind a scheduled failure.

The workflow can be started manually when the QEMU topologies need to run
outside the weekly schedule.
