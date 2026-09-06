alias a := audit
alias c := check
alias f := fmt
alias l := lock
alias p := pre-push
alias t := test
alias z := zizmor

alias cr := crawler-scenario
alias od := one-device
alias td := two-devices

stable := `cargo rbmt toolchains --stable`
export RBMT_LOG_LEVEL := env("RBMT_LOG_LEVEL", "progress")

_default:
    @echo "> rust-mikrotik"
    @echo "> A suite of Rust crates for simulating and interacting with MikroTik devices\n"
    @just --list

# Quality

[doc("Run cargo-audit across all lockfiles")]
[group("Quality")]
audit:
    @echo "Auditing Cargo.lock"
    cargo audit --file Cargo.lock
    @echo "\nAuditing Cargo-recent.lock"
    cargo audit --file Cargo-recent.lock
    @echo "\nAuditing Cargo-minimal.lock"
    cargo audit --file Cargo-minimal.lock

[doc("Check formatting, linting, and documentation")]
[group("Quality")]
check:
    cargo rbmt fmt --check
    cargo rbmt lint
    cargo rbmt docs

[doc("Format code")]
[group("Quality")]
fmt:
    cargo rbmt fmt

[doc("Run pre-push checks")]
[group("Quality")]
pre-push:
    @just lock
    @just check
    @just test
    @just shellcheck
    @just zizmor
    @just audit

[doc("Run ShellCheck")]
[group("Quality")]
shellcheck:
    @command -v shellcheck >/dev/null 2>&1 || { echo "shellcheck was not found on \$PATH" && exit 1; }
    find . -name '*.sh' -print -exec shellcheck {} +

[doc("Run Zizmor")]
[group("Quality")]
zizmor:
    zizmor .

# Documentation

[doc("Generate documentation")]
[group("Documentation")]
docs:
    cargo rbmt docs

[doc("Generate and open documentation")]
[group("Documentation")]
docs-open:
    cargo rbmt docs --open

# Testing

[doc("Generate code coverage (pass `open` to open the HTML report)")]
[env("CARGO_LLVM_COV_SETUP", "yes")]
[group("Testing")]
coverage open="":
    cargo +{{ stable }} llvm-cov {{ if open == "" { "" } else { "--open" } }} \
        --all-features \
        --html \
        --ignore-filename-regex '(^|/)test[.]rs$'
    cargo +{{ stable }} llvm-cov report \
        --lcov \
        --output-path target/llvm-cov/lcov.info \
        --ignore-filename-regex '(^|/)test[.]rs$'

[doc("Run all examples for one package")]
[group("Testing")]
examples package:
    cargo test -p {{ package }} --examples

[doc("Run one ignored QEMU runner scenario test")]
[group("Testing")]
scenario test:
    cargo rbmt -p mikrotik-qemu-runner run -- test --test scenarios {{ test }} -- --ignored --exact --nocapture

[doc("Run tests")]
[env("RBMT_LOG_LEVEL", "verbose")]
[group("Testing")]
test:
    cargo rbmt test

[doc("Run tests with lockfile and toolchain combinations")]
[env("RBMT_LOG_LEVEL", "verbose")]
[group("Testing")]
test-all:
    cargo rbmt test --toolchain msrv --lockfile minimal
    cargo rbmt test --toolchain stable --lockfile minimal
    cargo rbmt test --toolchain stable --lockfile recent

# Dependencies

[doc("Regenerate lockfiles")]
[group("Dependencies")]
lock:
    cargo rbmt lock

# Development

[doc("Run one rbmt task for one package")]
[group("Development")]
pkg package task:
    cargo rbmt -p {{ package }} {{ task }}

[doc("Run one cargo command through rbmt for one package")]
[group("Development")]
run package +args:
    cargo rbmt -p {{ package }} run -- {{ args }}

# Simulation

[doc("Run the crawler binary against a scenario manifest")]
[group("Simulation")]
crawl scenario="mikrotik-qemu-runner/scenarios/isp-network.toml":
    cargo run -p mikrotik-crawler --bin crawler -- --run-kind scenario --scenario {{ scenario }} --mode one-shot --protocol api

[doc("Run the crawler against the default QEMU runner scenario example")]
[group("Simulation")]
crawler-scenario:
    cargo run -p mikrotik-crawler --example qemu_runner_crawl

[doc("Spawn one CHR device through mikrotik-qemu-runner")]
[group("Simulation")]
one-device:
    cargo run -p mikrotik-qemu-runner --example one_device

[doc("Spawn two linked CHR devices through mikrotik-qemu-runner")]
[group("Simulation")]
two-devices:
    cargo run -p mikrotik-qemu-runner --example two_devices

# Setup

[doc("Install QEMU runner dependencies")]
[group("Setup")]
qemu-deps:
    brew install qemu

[doc("Install Rust and QEMU development dependencies")]
[group("Setup")]
setup:
    @just toolchains
    @just tools
    @just qemu-deps

[doc("Update stable and nightly toolchains")]
[group("Setup")]
toolchains:
    cargo rbmt toolchains --update-stable
    cargo rbmt toolchains --update-nightly

[doc("Install cargo-rbmt tools")]
[group("Setup")]
tools:
    cargo rbmt tools
