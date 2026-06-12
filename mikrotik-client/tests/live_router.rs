//! Optional live-router integration checks for print endpoint coverage.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use mikrotik_client::MikroTikClient;
use mikrotik_client::MikroTikClientConfig;
use mikrotik_client::Protocol;
use mikrotik_client::print_checks::PRINT_CHECK_FILTER_ENV;
use mikrotik_client::print_checks::PrintCheckFilter;
use mikrotik_client::print_checks::PrintCheckOptions;
use mikrotik_client::print_checks::run_all_print_checks;
use mikrotik_client::types::target::Credentials;

const LIVE_CREDS_PATH: &str = "tests/live_router_creds.toml";
const LIVE_CREDS_KEYS: [&str; 5] = ["address", "port", "username", "password", "protocol"];
const LIVE_ENABLE_ENV: &str = "MIKROTIK_LIVE";

#[tokio::test]
async fn live_router_print_endpoints() {
    if !live_enabled() {
        println!("skipping live router test: set {LIVE_ENABLE_ENV}=1 to run against {LIVE_CREDS_PATH}");
        return;
    }

    let Some(config) = live_config().expect("live router configuration should be readable") else {
        println!("skipping live router test: {LIVE_CREDS_PATH} is missing or incomplete");
        return;
    };

    let client = MikroTikClient::connect(config)
        .await
        .expect("live router should accept login");
    let filter = PrintCheckFilter::from_env();

    if let Some(pattern) = filter.pattern() {
        println!("filtering live router methods with {PRINT_CHECK_FILTER_ENV}={pattern}");
    }

    let report = run_all_print_checks(&client, &PrintCheckOptions::new().with_filter(filter)).await;
    report.assert_success();
}

fn live_enabled() -> bool {
    std::env::var(LIVE_ENABLE_ENV).is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn live_config() -> io::Result<Option<MikroTikClientConfig>> {
    let creds = read_creds_file(Path::new(env!("CARGO_MANIFEST_DIR")).join(LIVE_CREDS_PATH))?;

    if LIVE_CREDS_KEYS.iter().any(|key| !creds.contains_key(*key)) {
        return Ok(None);
    }

    let address = creds.get("address").expect("presence checked").to_owned();
    let port = creds
        .get("port")
        .expect("presence checked")
        .parse::<u16>()
        .expect("port should be a valid TCP port");
    let username = creds.get("username").expect("presence checked").to_owned();
    let password = creds.get("password").expect("presence checked").to_owned();
    let protocol = parse_protocol(creds.get("protocol").expect("presence checked"));

    Ok(Some(
        MikroTikClientConfig::new(
            address,
            protocol,
            Credentials {
                username,
                password: Some(password),
            },
        )
        .with_port(port),
    ))
}

fn read_creds_file(path: impl AsRef<Path>) -> io::Result<BTreeMap<String, String>> {
    let path = path.as_ref();

    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    fs::read_to_string(path).map(|contents| contents.lines().filter_map(parse_toml_line).collect::<BTreeMap<_, _>>())
}

fn parse_toml_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();

    if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
        return None;
    }

    let (key, value) = line.split_once('=')?;

    Some((key.trim().to_owned(), unquote(value.trim()).to_owned()))
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|value| value.strip_suffix('\'')))
        .unwrap_or(value)
}

fn parse_protocol(value: &str) -> Protocol {
    match value {
        "api" => Protocol::Api,
        "api-ssl" => Protocol::ApiSsl,
        protocol => panic!("protocol should be `api` or `api-ssl`, got `{protocol}`"),
    }
}
