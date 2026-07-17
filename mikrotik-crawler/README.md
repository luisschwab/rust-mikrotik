# mikrotik-crawler

<p>
    <a href="https://crates.io/crates/mikrotik-crawler"><img src="https://img.shields.io/crates/v/mikrotik-crawler.svg"/></a>
    <a href="https://docs.rs/mikrotik-crawler"><img src="https://img.shields.io/badge/docs.rs-mikrotik--crawler-blue"/></a>
    <a href="https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg"/></a>
</p>

Long-running read-only RouterOS discovery and snapshot collection.

This crate keeps a target registry in memory, refreshes `CollectedSnapshot` values
with Tokio background tasks, and emits snapshot events for observers.
