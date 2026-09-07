//! Compile checks for the feature-gated workspace crate re-exports.

#[cfg(any(
    feature = "client",
    feature = "common",
    feature = "crawler",
    feature = "graphviz",
    feature = "proto2",
    feature = "qemu-runner",
    feature = "types"
))]
use core::any::type_name;
#[cfg(feature = "common-tracing-subscriber")]
use core::any::type_name_of_val;

#[cfg(feature = "client")]
#[test]
fn reexports_client() {
    let _ = type_name::<mikrotik_meta::client::client::Client>();
}

#[cfg(feature = "common")]
#[test]
fn reexports_common() {
    let _ = type_name::<mikrotik_meta::common::row::Row>();
}

#[cfg(feature = "crawler")]
#[test]
fn reexports_crawler() {
    let _ = type_name::<mikrotik_meta::crawler::Crawler>();
}

#[cfg(feature = "graphviz")]
#[test]
fn reexports_graphviz() {
    let _ = type_name::<mikrotik_meta::graphviz::options::DotExportOptions>();
}

#[cfg(feature = "proto2")]
#[test]
fn reexports_proto2() {
    let _ = type_name::<mikrotik_meta::proto2::Connection>();
}

#[cfg(feature = "proto2-std")]
#[test]
fn forwards_proto2_std() {
    let _: std::collections::HashMap<u8, u8> = mikrotik_meta::proto2::HashMap::new();
}

#[cfg(feature = "qemu-runner")]
#[test]
fn reexports_qemu_runner() {
    let _ = type_name::<mikrotik_meta::qemu_runner::MikrotikD>();
}

#[cfg(feature = "types")]
#[test]
fn reexports_types() {
    let _ = type_name::<mikrotik_meta::types::RouterOsId>();
}

#[cfg(feature = "common-tracing-subscriber")]
#[test]
fn forwards_common_tracing_subscriber() {
    let _ = type_name_of_val(&mikrotik_meta::common::logging::init_tracing);
}
