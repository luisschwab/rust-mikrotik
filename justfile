alias a := audit
alias c := check
alias f := fmt
alias l := lock
alias t := test
alias z := zizmor
alias p := pre-push

alias cr := crawler-scenario
alias od := one-device
alias td := two-devices

export RBMT_LOG_LEVEL := env("RBMT_LOG_LEVEL", "verbose")

_default:
    @echo "> rust-mikrotik"
    @echo "> A suite of Rust crates for simulating and interacting with MikroTik devices\n"
    @just --list

[doc: "Run cargo-audit across all lockfiles"]
audit:
    @echo "Auditing Cargo.lock"
    cargo audit --file Cargo.lock
    @echo "\nAuditing Cargo-recent.lock"
    cargo audit --file Cargo-recent.lock
    @echo "\nAuditing Cargo-minimal.lock"
    cargo audit --file Cargo-minimal.lock

[doc: "Check Formatting, Linting and Documentation"]
check:
    cargo rbmt fmt --check
    cargo rbmt lint
    cargo rbmt docs

[doc: "Generate Documentation"]
docs:
    cargo rbmt docs

[doc: "Generate and Open Documentation"]
docs-open:
    cargo rbmt docs --open

[doc: "Format Code"]
fmt:
    cargo rbmt fmt

[doc: "Regenerate Lockfiles"]
lock:
    cargo rbmt lock

[doc: "Run Tests"]
test:
    cargo rbmt test

[doc: "Run Tests with Lockfile and Toolchain Combos"]
test-all:
    cargo rbmt test  --toolchain msrv --lockfile minimal
    cargo rbmt test  --toolchain stable --lockfile minimal
    cargo rbmt test  --toolchain stable --lockfile recent

[doc: "Install Rust and QEMU development dependencies"]
setup:
    @just toolchains
    @just tools
    @just qemu-deps

[doc: "Update Stable and Nightly Toolchains"]
toolchains:
    cargo rbmt toolchains --update-stable
    cargo rbmt toolchains --update-nightly

[doc: "Install cargo-rbmt Tools"]
tools:
    RBMT_LOG_LEVEL=progress cargo rbmt tools

[doc: "Install QEMU runner dependencies"]
qemu-deps:
    brew install qemu

[doc: "Run ShellCheck"]
shellcheck:
    @command -v shellcheck >/dev/null 2>&1 || { echo "shellcheck was not found on \$PATH" && exit 1; }
    find . -name '*.sh' -print -exec shellcheck {} +

[doc: "Run Zizmor"]
zizmor:
    zizmor .

[doc: "Run pre-push checks"]
pre-push:
    @just lock
    @just check
    @just test
    @just shellcheck
    @just zizmor
    @just audit

[doc: "Run one rbmt task for one package"]
pkg package task:
    cargo rbmt -p {{package}} {{task}}

[doc: "Run one cargo command through rbmt for one package"]
run package +args:
    cargo rbmt -p {{package}} run -- {{args}}

[doc: "Run all examples for one package"]
examples package:
    cargo test -p {{package}} --examples

[doc: "Run one ignored QEMU runner scenario test"]
scenario test:
    cargo rbmt -p mikrotik-qemu-runner run -- test --test scenarios {{test}} -- --ignored --exact --nocapture

[doc: "Run the crawler against the default QEMU runner scenario example"]
crawler-scenario:
    cargo run -p mikrotik-crawler --example qemu_runner_crawl

[doc: "Run the crawler binary against a scenario manifest"]
crawl scenario="mikrotik-qemu-runner/scenarios/isp-network.toml":
    cargo run -p mikrotik-crawler --bin crawler -- --run-kind scenario --scenario {{scenario}} --mode one-shot --protocol api

[doc: "Spawn one CHR device through mikrotik-qemu-runner"]
one-device:
    cargo run -p mikrotik-qemu-runner --example one_device

[doc: "Spawn two linked CHR devices through mikrotik-qemu-runner"]
two-devices:
    cargo run -p mikrotik-qemu-runner --example two_devices
