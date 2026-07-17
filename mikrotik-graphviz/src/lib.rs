//! Device graph types and Graphviz export.
//!
//! The graph is built from collected
//! [`mikrotik_types::device::RouterOsSnapshot`] values plus neighbor evidence for
//! devices that were seen but not crawled. It can be serialized as structured
//! data or rendered to DOT for external Graphviz tools.

pub mod constants;
pub mod error;
pub mod graph;
pub mod options;
pub mod render;
pub mod snapshot;
