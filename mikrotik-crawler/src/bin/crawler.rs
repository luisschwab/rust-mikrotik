//! Crawl a `RouterOS` topology and export run artifacts.

use core::fmt;
use core::net::IpAddr;
use core::net::SocketAddr;
use core::str::FromStr;
use core::time::Duration;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_dir;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use argh::FromArgs;
use mikrotik_client::builder::Protocol;
use mikrotik_crawler::AddressFamily;
use mikrotik_crawler::CrawlConfig;
use mikrotik_crawler::CrawlReport;
use mikrotik_crawler::Crawler;
use mikrotik_crawler::CrawlerService;
use mikrotik_crawler::CrawlerServiceConfig;
use mikrotik_crawler::CrawlerStateSnapshot;
use mikrotik_crawler::DEFAULT_COMMAND_TIMEOUT;
use mikrotik_crawler::DEFAULT_CONNECT_RETRIES;
use mikrotik_crawler::DEFAULT_CONNECT_TIMEOUT;
use mikrotik_crawler::RouterOsApiConnector;
use mikrotik_crawler::SnapshotClientConnector;
use mikrotik_crawler::resolver::DirectTargetResolver;
use mikrotik_crawler::resolver::StaticTargetResolver;
use mikrotik_crawler::resolver::TargetResolver;
use mikrotik_graphviz::constants::GRAPHVIZ_LAYERED_LAYOUT;
use mikrotik_graphviz::constants::GRAPHVIZ_RADIAL_LAYOUT;
use mikrotik_graphviz::constants::GRAPHVIZ_RECURSIVE_RADIAL_LAYOUT;
use mikrotik_graphviz::constants::GRAPHVIZ_SFDP_LAYOUT;
use mikrotik_graphviz::constants::GRAPHVIZ_TYPED_RADIAL_LAYOUT;
use mikrotik_graphviz::graph::model::NetworkGraph;
use mikrotik_graphviz::options::DotExportOptions;
use mikrotik_graphviz::options::GraphvizFormat;
use mikrotik_graphviz::options::GraphvizRenderOptions;
use mikrotik_graphviz::options::LinkFilter;
use mikrotik_graphviz::render::render_graphviz_artifact;
use mikrotik_graphviz::render::write_graphviz_interactive_html;
use mikrotik_graphviz::snapshot::GraphSnapshot;
use mikrotik_qemu_runner::Scenario;
use mikrotik_qemu_runner::ScenarioConf;
use mikrotik_types::target::Credentials;
use mikrotik_types::target::DeviceTarget;
use mikrotik_types::topology::InferredDevice;
use mikrotik_types::topology::InferredDeviceFailure;
use mikrotik_types::topology::NetworkNode;
use mikrotik_types::topology::NetworkNodeStatus;
use time::OffsetDateTime;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt;

/// Default recursive crawl depth for one-shot runs.
const DEFAULT_MAX_DEPTH: usize = 2;
/// Default maximum collected device count for one-shot runs.
const DEFAULT_MAX_DEVICES: usize = 1_000;
/// Default maximum concurrent snapshot jobs.
const DEFAULT_MAX_CONCURRENCY: usize = 16;
/// Default live artifact run root.
const DEFAULT_LIVE_RUNS_DIR: &str = "mikrotik-crawler/src/bin/runs/live";
/// Default QEMU runner scenario artifact run root.
const DEFAULT_SCENARIO_RUNS_DIR: &str = "mikrotik-crawler/src/bin/runs/scenario";
/// Default tracing directive for the crawler CLI.
const DEFAULT_LOG_FILTER: &str = "mikrotik_crawler=info";
/// Stable symlink name for the latest run under a run root.
const LATEST_RUN_SYMLINK: &str = "latest";
/// Worker stack size for large `RouterOS` snapshot futures.
const TOKIO_WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

/// CLI entrypoint.
fn main() -> Result<(), Box<dyn Error>> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(TOKIO_WORKER_STACK_SIZE)
        .build()?
        .block_on(run(argh::from_env()))
}

/// Run a crawl and export artifacts from parsed CLI arguments.
async fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let run_dir = args
        .outdir
        .clone()
        .unwrap_or_else(|| default_outdir(args.effective_run_kind()));
    fs::create_dir_all(&run_dir)?;
    let _log_guard = init_tracing(&run_dir);
    let scenario = maybe_spawn_scenario(&args).await?;

    let seeds = effective_seed_targets(&args, scenario.as_ref())?;
    let graph = match args.mode {
        RunMode::OneShot => run_one_shot(&args, seeds.clone()).await?,
        RunMode::Continuous => run_continuous(&args, seeds.clone()).await?,
    };

    let export_options = dot_export_options(&graph, &seeds, &args);
    let paths = ArtifactPaths::new(&run_dir);
    write_crawl_artifacts(&graph, &export_options, &paths, args.png, args.png_dpi.as_deref())?;
    let latest_path = update_latest_run_symlink(&run_dir)?;
    print_run_summary(&graph, &paths, &latest_path);
    Ok(())
}

/// Spawn a QEMU runner scenario when requested.
async fn maybe_spawn_scenario(args: &Args) -> Result<Option<Scenario>, Box<dyn Error>> {
    let Some(scenario_path) = &args.scenario else {
        return Ok(None);
    };
    let scenario_conf = ScenarioConf::read(scenario_path)?;
    let timeout = Duration::from_secs(args.scenario_ready_timeout_seconds);
    eprintln!("starting QEMU runner scenario from {}", scenario_path.display());
    let scenario = tokio::time::timeout(timeout, Scenario::new_with_conf(&scenario_conf))
        .await
        .map_err(|error| format!("QEMU runner scenario did not become ready within {timeout:?}: {error}"))??;
    Ok(Some(scenario))
}

/// Run one recursive crawl to completion.
async fn run_one_shot(args: &Args, seeds: Vec<DeviceTarget>) -> Result<NetworkGraph, Box<dyn Error>> {
    let factory = RouterOsApiConnector::new(args.protocol)
        .with_api_fallback()
        .with_connect_timeout(Duration::from_secs(args.connect_timeout_seconds))
        .with_command_timeout(Duration::from_secs(args.command_timeout_seconds));
    let mut crawler = Crawler::new(Arc::new(factory)).with_config(CrawlConfig {
        max_depth: args.max_depth,
        max_devices: args.max_devices,
        max_concurrency: args.max_concurrency,
        connect_retries: args.connect_retries,
        address_family: args.address_family,
    });
    if let Some(mappings) = &args.mappings {
        crawler = crawler.with_target_resolver(Arc::new(static_resolver(mappings)?));
    }

    let report = crawler.crawl_many(seeds).await?;
    print_failures(&report);
    Ok(report.graph)
}

/// Start the long-running crawler and export current state after Ctrl-C.
async fn run_continuous(args: &Args, seeds: Vec<DeviceTarget>) -> Result<NetworkGraph, Box<dyn Error>> {
    let config = CrawlerServiceConfig {
        seeds,
        snapshot_concurrency: args.max_concurrency,
        discovery_interval: Duration::from_secs(args.discovery_interval_seconds),
        snapshot_interval: Duration::from_secs(args.snapshot_interval_seconds),
        connect_timeout: Duration::from_secs(args.connect_timeout_seconds),
        command_timeout: Duration::from_secs(args.command_timeout_seconds),
        address_family: args.address_family,
        protocol: args.protocol,
    };
    let factory: Arc<dyn SnapshotClientConnector> = Arc::new(
        RouterOsApiConnector::new(args.protocol)
            .with_api_fallback()
            .with_connect_timeout(Duration::from_secs(args.connect_timeout_seconds))
            .with_command_timeout(Duration::from_secs(args.command_timeout_seconds)),
    );
    let target_resolver: Arc<dyn TargetResolver> = if let Some(mappings) = &args.mappings {
        Arc::new(static_resolver(mappings)?)
    } else {
        Arc::new(DirectTargetResolver)
    };
    let service = CrawlerService::start_with_parts(&config, &factory, &target_resolver);
    let handle = service.handle();

    eprintln!("crawler running; press Ctrl-C to stop and export artifacts");
    tokio::signal::ctrl_c().await?;
    eprintln!("stopping crawler and exporting current state");

    let state = handle.state().await;
    drop(service);
    Ok(graph_from_state(&state))
}

/// Build a graph from continuous crawler state and failed target markers.
fn graph_from_state(state: &CrawlerStateSnapshot) -> NetworkGraph {
    let snapshots = state.snapshots.values().map(GraphSnapshot::from).collect::<Vec<_>>();
    let mut graph = NetworkGraph::from_snapshots(&snapshots);
    add_failed_targets_to_graph(&mut graph, state);
    graph
}

/// Add current failed crawler targets as inferred graph nodes.
fn add_failed_targets_to_graph(graph: &mut NetworkGraph, state: &CrawlerStateSnapshot) {
    let names = failure_names_from_state(state);
    for (address, message) in &state.failures {
        if graph
            .node_key_for_target_address(&address.ip().to_string())
            .or_else(|| graph.node_key_for_target_address(&address.to_string()))
            .is_some()
        {
            continue;
        }
        graph.nodes.push(NetworkNode {
            key: address.to_string().into(),
            status: NetworkNodeStatus::Inferred,
            role: None,
            target_address: None,
            management_addresses: Vec::new(),
            snapshot: None,
            inferred: Some(InferredDevice {
                management_address: Some(address.ip()),
                identity: names
                    .get(&address.to_string())
                    .cloned()
                    .or_else(|| Some(address.ip().to_string())),
                board: None,
                platform: None,
                version: None,
                mac_address: None,
                failure: Some(failure_kind_from_message(message)),
            }),
        });
    }
}

/// Return best-effort inferred names for failed targets.
fn failure_names_from_state(state: &CrawlerStateSnapshot) -> BTreeMap<String, String> {
    let mut names = BTreeMap::new();
    for snapshot in state.snapshots.values() {
        for neighbor in &snapshot.ip.neighbors.data {
            let Some(address) = neighbor.management_address() else {
                continue;
            };
            let Some(identity) = neighbor.identity.as_ref().filter(|identity| !identity.is_empty()) else {
                continue;
            };
            names.entry(address.to_string()).or_insert_with(|| identity.clone());
            names
                .entry(SocketAddr::new(address, snapshot.target_address.port()).to_string())
                .or_insert_with(|| identity.clone());
        }
    }
    names
}

/// Infer a graph failure marker from one crawler error string.
fn failure_kind_from_message(message: &str) -> InferredDeviceFailure {
    let lower = message.to_ascii_lowercase();
    if lower.contains("authentication failed")
        || lower.contains("invalid user name or password")
        || lower == "invalid credentials"
    {
        InferredDeviceFailure::WrongCredentials
    } else if lower.contains("connection refused") || lower == "api refused connection" {
        InferredDeviceFailure::ApiRefused
    } else {
        InferredDeviceFailure::Unreachable
    }
}

/// Build graph export options for a finished run.
fn dot_export_options(graph: &NetworkGraph, seeds: &[DeviceTarget], args: &Args) -> DotExportOptions {
    let seed_nodes = seeds
        .iter()
        .filter_map(|seed| graph.node_key_for_target_address(&seed.address.ip().to_string()))
        .map(|key| key.as_str().to_owned())
        .collect::<Vec<_>>();
    DotExportOptions {
        root_node: seed_nodes.first().cloned(),
        seed_nodes: seed_nodes.clone(),
        owned_bgp_nodes: seed_nodes,
        link_filter: args.link_filter,
        hide_link_tables: !args.show_link_tables,
        ..DotExportOptions::for_layout(args.layout.clone())
    }
}

/// Artifact paths produced by one crawl run.
#[derive(Debug, Clone)]
struct ArtifactPaths {
    /// Primary DOT output.
    dot: PathBuf,
    /// Primary SVG output.
    svg: PathBuf,
    /// Primary PNG output.
    png: PathBuf,
    /// Interactive HTML output.
    interactive_html: PathBuf,
}

impl ArtifactPaths {
    /// Build all artifact paths under one output directory.
    fn new(outdir: &Path) -> Self {
        Self {
            dot: outdir.join("topology.dot"),
            svg: outdir.join("topology.svg"),
            png: outdir.join("topology.png"),
            interactive_html: outdir.join("topology.interactive.html"),
        }
    }
}

/// Write DOT, SVG, interactive HTML, and optional PNG artifacts for one graph.
fn write_crawl_artifacts(
    graph: &NetworkGraph,
    export_options: &DotExportOptions,
    paths: &ArtifactPaths,
    png: bool,
    png_dpi: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    fs::write(&paths.dot, graph.to_graphviz_dot_with_options(export_options))?;
    render_graphviz_outputs(paths, png, png_dpi)?;
    Ok(())
}

/// Render Graphviz-derived SVG, interactive HTML, and optional PNG artifacts.
fn render_graphviz_outputs(paths: &ArtifactPaths, png: bool, png_dpi: Option<&str>) -> Result<(), Box<dyn Error>> {
    if !has_graphviz_dot() {
        return Err("Graphviz 'dot' executable is required to export interactive HTML".into());
    }
    let mut render_options = GraphvizRenderOptions::default();
    if let Some(png_dpi) = png_dpi {
        png_dpi.clone_into(&mut render_options.png_dpi);
    }
    render_graphviz_artifact(GraphvizFormat::Svg, &paths.dot, &paths.svg, &render_options)?;
    write_graphviz_interactive_html(&paths.svg, &paths.interactive_html)?;
    if png {
        render_graphviz_artifact(GraphvizFormat::Png, &paths.dot, &paths.png, &render_options)?;
    }
    Ok(())
}

/// Return whether the Graphviz `dot` command is available.
fn has_graphviz_dot() -> bool {
    Command::new("dot")
        .arg("-V")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Print a concise run summary.
fn print_run_summary(graph: &NetworkGraph, paths: &ArtifactPaths, latest_path: &Path) {
    let collected = graph
        .nodes
        .iter()
        .filter(|node| matches!(node.status, NetworkNodeStatus::Collected))
        .count();
    let inferred = graph.nodes.len().saturating_sub(collected);

    println!("nodes={} collected={collected} inferred={inferred}", graph.nodes.len());
    println!("edges={}", graph.edges.len());
    println!("dot={}", paths.dot.display());
    println!("svg={}", paths.svg.display());
    println!("interactive_html={}", paths.interactive_html.display());
    if paths.png.exists() {
        println!("png={}", paths.png.display());
    }
    println!("latest={}", latest_path.display());
}

/// Print failed one-shot crawl targets.
fn print_failures(report: &CrawlReport) {
    for failure in &report.failed_targets {
        println!("failure {} {}", failure.address, failure.error);
    }
}

/// crawl a `RouterOS` topology and export Graphviz artifacts.
#[derive(Debug, FromArgs)]
struct Args {
    /// seed host or IP address with a port; repeat for multiple seeds.
    #[argh(option, from_str_fn(parse_target_address))]
    seed: Vec<SocketAddr>,
    /// run mode: one-shot or continuous.
    #[argh(option, default = "RunMode::OneShot", from_str_fn(parse_run_mode))]
    mode: RunMode,
    /// run folder family: live or scenario.
    #[argh(option, default = "RunKind::Live", from_str_fn(parse_run_kind))]
    run_kind: RunKind,
    /// QEMU runner scenario TOML to spawn before crawling.
    #[argh(option)]
    scenario: Option<PathBuf>,
    /// seconds to wait for a spawned QEMU runner scenario to become API-ready.
    #[argh(option, default = "900")]
    scenario_ready_timeout_seconds: u64,
    /// preferred routerOS API protocol: api or api-ssl.
    #[argh(option, default = "Protocol::ApiSsl", from_str_fn(parse_protocol))]
    protocol: Protocol,
    /// routerOS API username.
    #[argh(option)]
    user: String,
    /// routerOS API password.
    #[argh(option, default = "String::new()")]
    password: String,
    /// artifact output directory.
    #[argh(option)]
    outdir: Option<PathBuf>,
    /// static discovered-address to target-address mappings.
    #[argh(option, short = 'm')]
    mappings: Option<String>,
    /// maximum recursive discovery depth for one-shot mode.
    #[argh(option, default = "DEFAULT_MAX_DEPTH")]
    max_depth: usize,
    /// maximum collected devices for one-shot mode.
    #[argh(option, default = "DEFAULT_MAX_DEVICES")]
    max_devices: usize,
    /// maximum concurrent device snapshot jobs.
    #[argh(option, default = "DEFAULT_MAX_CONCURRENCY")]
    max_concurrency: usize,
    /// seconds allowed for one connection attempt.
    #[argh(option, default = "DEFAULT_CONNECT_TIMEOUT.as_secs()")]
    connect_timeout_seconds: u64,
    /// seconds allowed for one `RouterOS` print command.
    #[argh(option, default = "DEFAULT_COMMAND_TIMEOUT.as_secs()")]
    command_timeout_seconds: u64,
    /// extra attempts for one-shot targets that time out.
    #[argh(option, default = "DEFAULT_CONNECT_RETRIES")]
    connect_retries: usize,
    /// seconds between continuous-mode discovery passes.
    #[argh(option, default = "30")]
    discovery_interval_seconds: u64,
    /// seconds between continuous-mode snapshot passes.
    #[argh(option, default = "60")]
    snapshot_interval_seconds: u64,
    /// address family for recursively discovered neighbor targets.
    #[argh(option, default = "AddressFamily::Ipv4", from_str_fn(parse_address_family))]
    address_family: AddressFamily,
    /// link filter for rendered artifacts.
    #[argh(option, default = "LinkFilter::Routing", from_str_fn(parse_link_filter))]
    link_filter: LinkFilter,
    /// show visible link detail tables.
    #[argh(switch)]
    show_link_tables: bool,
    /// graphviz layout engine.
    #[argh(option, default = "GRAPHVIZ_SFDP_LAYOUT.to_owned()", from_str_fn(parse_layout))]
    layout: String,
    /// override PNG DPI.
    #[argh(option)]
    png_dpi: Option<String>,
    /// render raster PNG artifacts.
    #[argh(switch)]
    png: bool,
}

impl Args {
    /// Return the effective output run family.
    const fn effective_run_kind(&self) -> RunKind {
        if self.scenario.is_some() {
            RunKind::Scenario
        } else {
            self.run_kind
        }
    }
}

/// Crawler execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    /// Crawl once and export after the recursive crawl completes.
    OneShot,
    /// Keep crawling until Ctrl-C, then export current state.
    Continuous,
}

/// Output run directory family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunKind {
    /// Live network run.
    Live,
    /// QEMU runner scenario run.
    Scenario,
}

/// Build seed targets from parsed addresses.
fn build_targets(seed_addresses: &[SocketAddr], credentials: &Credentials) -> Result<Vec<DeviceTarget>, String> {
    if seed_addresses.is_empty() {
        return Err("at least one --seed is required".to_owned());
    }
    Ok(seed_addresses
        .iter()
        .copied()
        .map(|address| DeviceTarget {
            address,
            credentials: credentials.clone(),
        })
        .collect())
}

/// Return explicit CLI seeds or targets returned by a spawned QEMU runner scenario.
fn effective_seed_targets(args: &Args, scenario: Option<&Scenario>) -> Result<Vec<DeviceTarget>, String> {
    if !args.seed.is_empty() {
        return build_targets(&args.seed, &build_credentials(args)?);
    }
    let Some(scenario) = scenario else {
        return Err("at least one --seed is required".to_owned());
    };
    let targets = scenario.targets();
    if targets.is_empty() {
        return Err("spawned QEMU runner scenario did not expose any API targets".to_owned());
    }
    Ok(targets)
}

/// Return the default timestamped output directory.
fn default_outdir(run_kind: RunKind) -> PathBuf {
    let root = match run_kind {
        RunKind::Live => DEFAULT_LIVE_RUNS_DIR,
        RunKind::Scenario => DEFAULT_SCENARIO_RUNS_DIR,
    };
    PathBuf::from(root).join(local_timestamp())
}

/// Return a UTC timestamp in run-directory format.
fn local_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

/// Initialize CLI logging to stderr and the run directory.
fn init_tracing(outdir: &Path) -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::never(outdir, "crawl.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::new(DEFAULT_LOG_FILTER);
    if let Err(error) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_writer.and(io::stderr))
        .try_init()
    {
        eprintln!("failed to initialize crawler logging: {error}");
    }
    guard
}

/// Parse a named value with a useful error.
fn parse_named_value<T>(name: &str, value: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    value.parse().map_err(|error| format!("{name} is invalid: {error}"))
}

/// Parse a run mode.
fn parse_run_mode(value: &str) -> Result<RunMode, String> {
    match value {
        "one-shot" | "oneshot" | "once" => Ok(RunMode::OneShot),
        "continuous" | "watch" | "service" => Ok(RunMode::Continuous),
        _ => Err("--mode must be one-shot or continuous".to_owned()),
    }
}

/// Parse a run directory family.
fn parse_run_kind(value: &str) -> Result<RunKind, String> {
    match value {
        "live" => Ok(RunKind::Live),
        "scenario" => Ok(RunKind::Scenario),
        _ => Err("--run-kind must be live or scenario".to_owned()),
    }
}

/// Parse a protocol accepted by the crawler.
fn parse_protocol(value: &str) -> Result<Protocol, String> {
    match value {
        "api" => Ok(Protocol::Api),
        "api-ssl" => Ok(Protocol::ApiSsl),
        _ => Err("--protocol must be api or api-ssl".to_owned()),
    }
}

/// Parse an address-family filter.
fn parse_address_family(value: &str) -> Result<AddressFamily, String> {
    match value {
        "any" => Ok(AddressFamily::Any),
        "ipv4" => Ok(AddressFamily::Ipv4),
        "ipv6" => Ok(AddressFamily::Ipv6),
        _ => Err("--address-family must be any, ipv4, or ipv6".to_owned()),
    }
}

/// Parse a link filter argument.
fn parse_link_filter(value: &str) -> Result<LinkFilter, String> {
    match value {
        "all" => Ok(LinkFilter::All),
        "routing" => Ok(LinkFilter::Routing),
        "physical" => Ok(LinkFilter::PhysicalOnly),
        "bgp" => Ok(LinkFilter::BgpOnly),
        _ => Err("--link-filter must be all, routing, physical, or bgp".to_owned()),
    }
}

/// Parse a Graphviz layout argument.
fn parse_layout(value: &str) -> Result<String, String> {
    match value {
        "twopi" | "radial" => Ok(GRAPHVIZ_RADIAL_LAYOUT.to_owned()),
        "typed-radial" | "sector-radial" | "sectors" => Ok(GRAPHVIZ_TYPED_RADIAL_LAYOUT.to_owned()),
        "sfdp" | "force" | "force-directed" => Ok(GRAPHVIZ_SFDP_LAYOUT.to_owned()),
        "recursive-radial" | "recursive" | "tree-radial" => Ok(GRAPHVIZ_RECURSIVE_RADIAL_LAYOUT.to_owned()),
        "dot" | "layered" => Ok(GRAPHVIZ_LAYERED_LAYOUT.to_owned()),
        _ => Err(
            "--layout must be sfdp/force, twopi/radial, typed-radial/sectors, recursive-radial, or dot/layered"
                .to_owned(),
        ),
    }
}

/// Build credentials from CLI flags.
fn build_credentials(args: &Args) -> Result<Credentials, String> {
    if args.user.is_empty() {
        return Err("--user must not be empty".to_owned());
    }
    Ok(Credentials {
        username: args.user.clone(),
        password: Some(args.password.clone()),
    })
}

/// Parse a connectable target address with an explicit port.
fn parse_target_address(value: &str) -> Result<SocketAddr, String> {
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (host, rest) = rest
            .split_once(']')
            .ok_or_else(|| "IPv6 values must use [addr]:port".to_owned())?;
        let port = rest
            .strip_prefix(':')
            .ok_or_else(|| "target address must include a port".to_owned())?;
        (host, port)
    } else {
        value
            .rsplit_once(':')
            .ok_or_else(|| "target address must include a port".to_owned())?
    };

    if host.is_empty() {
        return Err("target address host must not be empty".to_owned());
    }
    parse_named_value::<u16>("target address port", port)?;
    value
        .parse()
        .map_err(|error| format!("target address must be an IP socket address: {error}"))
}

/// Parse static resolver mappings.
fn static_resolver(mappings: &str) -> Result<StaticTargetResolver, String> {
    let mut resolver = StaticTargetResolver::new();
    for mapping in mappings.split(',').filter(|mapping| !mapping.trim().is_empty()) {
        let (discovered, target) = mapping
            .split_once('=')
            .ok_or_else(|| "mapping must be discovered-ip=target-address".to_owned())?;
        let discovered = discovered
            .trim()
            .parse::<IpAddr>()
            .map_err(|error| format!("discovered address is invalid: {error}"))?;
        resolver = resolver.with_target(discovered, target.trim());
    }
    Ok(resolver)
}

/// Update a sibling `latest` symlink so it points at one completed run directory.
fn update_latest_run_symlink(run_dir: impl AsRef<Path>) -> io::Result<PathBuf> {
    let run_dir = run_dir.as_ref();
    let parent = run_dir.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "run directory must have a parent for the latest symlink",
        )
    })?;
    let link_path = parent.join(LATEST_RUN_SYMLINK);
    replace_symlink(&link_path, &run_dir.canonicalize()?)?;
    Ok(link_path)
}

/// Replace one symlink path with a link to `target`.
fn replace_symlink(link_path: &Path, target: &Path) -> io::Result<()> {
    match fs::symlink_metadata(link_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
            fs::remove_file(link_path)?;
        }
        Ok(metadata) if metadata.is_dir() => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} exists and is a directory", link_path.display()),
            ));
        }
        Ok(_) => {
            fs::remove_file(link_path)?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    create_dir_symlink(target, link_path)
}

/// Create a directory symlink.
#[cfg(unix)]
fn create_dir_symlink(target: &Path, link_path: &Path) -> io::Result<()> {
    symlink(target, link_path)
}

/// Create a directory symlink.
#[cfg(windows)]
fn create_dir_symlink(target: &Path, link_path: &Path) -> io::Result<()> {
    symlink_dir(target, link_path)
}

#[cfg(test)]
mod tests {
    use std::process;

    use mikrotik_types::api::ip::Neighbor;
    use mikrotik_types::device::RouterOsSnapshot;

    use super::*;

    fn args() -> Args {
        Args {
            seed: vec!["192.0.2.1:8728".parse().unwrap()],
            mode: RunMode::OneShot,
            run_kind: RunKind::Live,
            scenario: None,
            scenario_ready_timeout_seconds: 1,
            protocol: Protocol::Api,
            user: "observer".to_owned(),
            password: "secret".to_owned(),
            outdir: None,
            mappings: None,
            max_depth: DEFAULT_MAX_DEPTH,
            max_devices: DEFAULT_MAX_DEVICES,
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
            connect_timeout_seconds: DEFAULT_CONNECT_TIMEOUT.as_secs(),
            command_timeout_seconds: DEFAULT_COMMAND_TIMEOUT.as_secs(),
            connect_retries: DEFAULT_CONNECT_RETRIES,
            discovery_interval_seconds: 30,
            snapshot_interval_seconds: 60,
            address_family: AddressFamily::Ipv4,
            link_filter: LinkFilter::Routing,
            show_link_tables: false,
            layout: GRAPHVIZ_SFDP_LAYOUT.to_owned(),
            png_dpi: None,
            png: false,
        }
    }

    fn temporary_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("mikrotik-crawler-cli-{}-{name}", process::id()))
    }

    #[test]
    fn named_argument_parsers_cover_aliases_and_errors() {
        for value in ["one-shot", "oneshot", "once"] {
            assert_eq!(parse_run_mode(value).unwrap(), RunMode::OneShot);
        }
        for value in ["continuous", "watch", "service"] {
            assert_eq!(parse_run_mode(value).unwrap(), RunMode::Continuous);
        }
        assert!(parse_run_mode("invalid").is_err());

        assert_eq!(parse_run_kind("live").unwrap(), RunKind::Live);
        assert_eq!(parse_run_kind("scenario").unwrap(), RunKind::Scenario);
        assert!(parse_run_kind("invalid").is_err());
        assert_eq!(parse_protocol("api").unwrap(), Protocol::Api);
        assert_eq!(parse_protocol("api-ssl").unwrap(), Protocol::ApiSsl);
        assert!(parse_protocol("ssh").is_err());
        assert_eq!(parse_address_family("any").unwrap(), AddressFamily::Any);
        assert_eq!(parse_address_family("ipv4").unwrap(), AddressFamily::Ipv4);
        assert_eq!(parse_address_family("ipv6").unwrap(), AddressFamily::Ipv6);
        assert!(parse_address_family("ip").is_err());
        assert_eq!(parse_link_filter("all").unwrap(), LinkFilter::All);
        assert_eq!(parse_link_filter("routing").unwrap(), LinkFilter::Routing);
        assert_eq!(parse_link_filter("physical").unwrap(), LinkFilter::PhysicalOnly);
        assert_eq!(parse_link_filter("bgp").unwrap(), LinkFilter::BgpOnly);
        assert!(parse_link_filter("none").is_err());

        for value in ["twopi", "radial"] {
            assert_eq!(parse_layout(value).unwrap(), GRAPHVIZ_RADIAL_LAYOUT);
        }
        for value in ["typed-radial", "sector-radial", "sectors"] {
            assert_eq!(parse_layout(value).unwrap(), GRAPHVIZ_TYPED_RADIAL_LAYOUT);
        }
        for value in ["sfdp", "force", "force-directed"] {
            assert_eq!(parse_layout(value).unwrap(), GRAPHVIZ_SFDP_LAYOUT);
        }
        for value in ["recursive-radial", "recursive", "tree-radial"] {
            assert_eq!(parse_layout(value).unwrap(), GRAPHVIZ_RECURSIVE_RADIAL_LAYOUT);
        }
        for value in ["dot", "layered"] {
            assert_eq!(parse_layout(value).unwrap(), GRAPHVIZ_LAYERED_LAYOUT);
        }
        assert!(parse_layout("invalid").is_err());
        assert_eq!(parse_named_value::<u16>("port", "8728").unwrap(), 8728);
        assert!(parse_named_value::<u16>("port", "invalid").is_err());
    }

    #[test]
    fn argh_populates_every_cli_option() {
        let parsed = Args::from_args(
            &["crawler"],
            &[
                "--seed",
                "192.0.2.10:18728",
                "--mode",
                "continuous",
                "--run-kind",
                "scenario",
                "--scenario",
                "scenario.toml",
                "--scenario-ready-timeout-seconds",
                "42",
                "--protocol",
                "api",
                "--user",
                "observer",
                "--password",
                "secret",
                "--outdir",
                "artifacts",
                "--mappings",
                "192.0.2.20=127.0.0.1:28728",
                "--max-depth",
                "4",
                "--max-devices",
                "50",
                "--max-concurrency",
                "8",
                "--connect-timeout-seconds",
                "7",
                "--command-timeout-seconds",
                "9",
                "--connect-retries",
                "2",
                "--discovery-interval-seconds",
                "11",
                "--snapshot-interval-seconds",
                "13",
                "--address-family",
                "any",
                "--link-filter",
                "physical",
                "--show-link-tables",
                "--layout",
                "layered",
                "--png-dpi",
                "144",
                "--png",
            ],
        )
        .unwrap();

        assert_eq!(parsed.seed, ["192.0.2.10:18728".parse().unwrap()]);
        assert_eq!(parsed.mode, RunMode::Continuous);
        assert_eq!(parsed.effective_run_kind(), RunKind::Scenario);
        assert_eq!(parsed.scenario_ready_timeout_seconds, 42);
        assert_eq!(parsed.protocol, Protocol::Api);
        assert_eq!(parsed.user, "observer");
        assert_eq!(parsed.password, "secret");
        assert_eq!(parsed.outdir, Some(PathBuf::from("artifacts")));
        assert_eq!(parsed.max_depth, 4);
        assert_eq!(parsed.max_devices, 50);
        assert_eq!(parsed.max_concurrency, 8);
        assert_eq!(parsed.connect_timeout_seconds, 7);
        assert_eq!(parsed.command_timeout_seconds, 9);
        assert_eq!(parsed.connect_retries, 2);
        assert_eq!(parsed.discovery_interval_seconds, 11);
        assert_eq!(parsed.snapshot_interval_seconds, 13);
        assert_eq!(parsed.address_family, AddressFamily::Any);
        assert_eq!(parsed.link_filter, LinkFilter::PhysicalOnly);
        assert!(parsed.show_link_tables);
        assert_eq!(parsed.layout, GRAPHVIZ_LAYERED_LAYOUT);
        assert_eq!(parsed.png_dpi.as_deref(), Some("144"));
        assert!(parsed.png);
    }

    #[test]
    fn target_address_parser_requires_an_ip_and_explicit_port() {
        assert_eq!(
            parse_target_address("192.0.2.1:8728").unwrap(),
            "192.0.2.1:8728".parse().unwrap()
        );
        assert_eq!(
            parse_target_address("[2001:db8::1]:8729").unwrap(),
            "[2001:db8::1]:8729".parse().unwrap()
        );
        for invalid in [
            "192.0.2.1",
            "[2001:db8::1",
            "[2001:db8::1]",
            ":8728",
            "router.example:8728",
            "192.0.2.1:invalid",
            "192.0.2.1:70000",
        ] {
            assert!(parse_target_address(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn credentials_targets_and_run_kinds_validate_inputs() {
        let mut args = args();
        let credentials = build_credentials(&args).unwrap();
        assert_eq!(credentials.username, "observer");
        assert_eq!(credentials.password.as_deref(), Some("secret"));
        assert_eq!(
            build_targets(&args.seed, &credentials).unwrap()[0].address,
            args.seed[0]
        );
        assert!(build_targets(&[], &credentials).is_err());

        let targets = effective_seed_targets(&args, None).unwrap();
        assert_eq!(targets.len(), 1);
        args.seed.clear();
        assert!(effective_seed_targets(&args, None).is_err());
        args.user.clear();
        args.seed.push("192.0.2.1:8728".parse().unwrap());
        assert!(effective_seed_targets(&args, None).is_err());

        args.scenario = Some(PathBuf::from("scenario.toml"));
        assert_eq!(args.effective_run_kind(), RunKind::Scenario);
        assert!(default_outdir(RunKind::Live).starts_with(DEFAULT_LIVE_RUNS_DIR));
        assert!(default_outdir(RunKind::Scenario).starts_with(DEFAULT_SCENARIO_RUNS_DIR));
        assert_eq!(local_timestamp().len(), 15);
    }

    #[test]
    fn static_mapping_parser_ignores_blanks_and_resolves_valid_entries() {
        let resolver = static_resolver("192.0.2.2=127.0.0.1:18728,, 2001:db8::2=[::1]:28728").unwrap();
        let credentials = Credentials {
            username: "observer".to_owned(),
            password: None,
        };
        let resolved = resolver
            .resolve(
                "192.0.2.2".parse().unwrap(),
                &credentials,
                &RouterOsSnapshot::default(),
                &Neighbor::default(),
            )
            .unwrap();
        assert_eq!(resolved.address, "127.0.0.1:18728".parse().unwrap());
        assert!(static_resolver("missing-separator").is_err());
        assert!(static_resolver("not-an-ip=127.0.0.1:8728").is_err());
    }

    #[test]
    fn graph_state_projection_adds_failures_and_selects_export_seeds() {
        let wrong_credentials = "192.0.2.1:8728".parse().unwrap();
        let refused = "192.0.2.2:8728".parse().unwrap();
        let unreachable = "192.0.2.3:8728".parse().unwrap();
        let mut state = CrawlerStateSnapshot::default();
        state
            .failures
            .insert(wrong_credentials, "authentication failed".to_owned());
        state.failures.insert(refused, "connection refused".to_owned());
        state.failures.insert(unreachable, "timed out".to_owned());

        let mut graph = graph_from_state(&state);
        assert_eq!(graph.nodes.len(), 3);
        let failures = graph
            .nodes
            .iter()
            .map(|node| node.inferred.as_ref().unwrap().failure.unwrap())
            .collect::<Vec<_>>();
        assert!(failures.contains(&InferredDeviceFailure::WrongCredentials));
        assert!(failures.contains(&InferredDeviceFailure::ApiRefused));
        assert!(failures.contains(&InferredDeviceFailure::Unreachable));
        assert_eq!(
            failure_kind_from_message("invalid user name or password"),
            InferredDeviceFailure::WrongCredentials
        );
        assert_eq!(
            failure_kind_from_message("invalid credentials"),
            InferredDeviceFailure::WrongCredentials
        );
        assert_eq!(
            failure_kind_from_message("api refused connection"),
            InferredDeviceFailure::ApiRefused
        );

        let cli_args = args();
        let seeds = build_targets(&[wrong_credentials], &build_credentials(&cli_args).unwrap()).unwrap();
        assert!(dot_export_options(&graph, &seeds, &cli_args).root_node.is_none());
        graph.nodes[0].target_address = Some(wrong_credentials);
        graph.nodes[0].management_addresses.push(wrong_credentials.ip());
        let options = dot_export_options(&graph, &seeds, &cli_args);
        assert!(options.root_node.is_some());
        assert_eq!(options.seed_nodes, options.owned_bgp_nodes);

        let existing_len = graph.nodes.len();
        add_failed_targets_to_graph(&mut graph, &state);
        assert_eq!(graph.nodes.len(), existing_len + 2);
        assert!(failure_names_from_state(&state).is_empty());
    }

    #[test]
    fn artifact_paths_and_symlink_replacement_cover_files_and_directories() {
        let root = temporary_root("symlinks");
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();

        let paths = ArtifactPaths::new(&first);
        assert_eq!(paths.dot, first.join("topology.dot"));
        assert_eq!(paths.svg, first.join("topology.svg"));
        assert_eq!(paths.png, first.join("topology.png"));
        assert_eq!(paths.interactive_html, first.join("topology.interactive.html"));

        let latest = update_latest_run_symlink(&first).unwrap();
        assert_eq!(latest, root.join(LATEST_RUN_SYMLINK));
        assert_eq!(fs::canonicalize(&latest).unwrap(), first.canonicalize().unwrap());
        replace_symlink(&latest, &second).unwrap();
        assert_eq!(fs::canonicalize(&latest).unwrap(), second.canonicalize().unwrap());

        fs::remove_file(&latest).unwrap();
        fs::write(&latest, b"old marker").unwrap();
        replace_symlink(&latest, &first).unwrap();
        assert_eq!(fs::canonicalize(&latest).unwrap(), first.canonicalize().unwrap());

        fs::remove_file(&latest).unwrap();
        fs::create_dir(&latest).unwrap();
        assert_eq!(
            replace_symlink(&latest, &first).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            update_latest_run_symlink(Path::new("/")).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn one_shot_cli_run_with_zero_device_limit_writes_all_artifacts() {
        if !has_graphviz_dot() {
            return;
        }
        let root = temporary_root("run");
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        let run_dir = root.join("run-1");
        let mut cli_args = args();
        cli_args.outdir = Some(run_dir.clone());
        cli_args.max_devices = 0;
        cli_args.mappings = Some(String::new());
        cli_args.png = true;
        cli_args.png_dpi = Some("96".to_owned());

        assert!(maybe_spawn_scenario(&cli_args).await.unwrap().is_none());
        run(cli_args).await.unwrap();

        let paths = ArtifactPaths::new(&run_dir);
        assert!(paths.dot.exists());
        assert!(paths.svg.exists());
        assert!(paths.png.exists());
        assert!(paths.interactive_html.exists());
        assert!(root.join(LATEST_RUN_SYMLINK).exists());
        print_run_summary(&NetworkGraph::default(), &paths, &root.join(LATEST_RUN_SYMLINK));
        print_failures(&CrawlReport {
            graph: NetworkGraph::default(),
            failed_targets: vec![mikrotik_crawler::CrawlFailure {
                address: "192.0.2.1:8728".to_owned(),
                error: "unreachable".to_owned(),
            }],
        });
        fs::remove_dir_all(root).unwrap();
    }
}
