//! Execute one raw `RouterOS` API command and print returned rows.
//!
//! Usage:
//!
//! ```text
//! cargo run -p mikrotik-client --example raw_call -- <device_address> <protocol> <user> <password> <command>
//!
//! cargo run -p mikrotik-client --example raw_call -- <device_address> <protocol> <user> <password> /system/resource/print
//! cargo run -p mikrotik-client --example raw_call -- <device_address> <protocol> <user> <password> /export terse
//! ```

use std::env;
use std::process;

use mikrotik_client::builder::ClientBuilder;
use mikrotik_client::builder::Protocol;
use mikrotik_client::client::Client;
use mikrotik_client::error::Result;
use mikrotik_client::types::target::Credentials;
use mikrotik_common::redaction::redact_command_row;

/// Parsed example arguments.
#[derive(Clone, PartialEq, Eq)]
struct Args {
    /// Router host name or IP address.
    host: String,
    /// `RouterOS` API protocol.
    protocol: Protocol,
    /// `RouterOS` username.
    username: String,
    /// `RouterOS` password.
    password: String,
    /// Raw command path.
    command: String,
    /// Raw attributes as `key=value` pairs or flag names.
    attributes: Vec<String>,
}

impl Args {
    /// Parse command-line arguments.
    fn parse() -> Self {
        let mut args = env::args().skip(1);
        let host = args.next().unwrap_or_else(|| usage());
        let protocol = parse_protocol(&args.next().unwrap_or_else(|| usage()));
        let username = args.next().unwrap_or_else(|| usage());
        let password = args.next().unwrap_or_else(|| usage());
        let command = args.next().unwrap_or_else(|| usage());
        let attributes = args.collect();

        Self {
            host,
            protocol,
            username,
            password,
            command,
            attributes,
        }
    }
}

/// Parse the protocol argument.
fn parse_protocol(protocol: &str) -> Protocol {
    match protocol {
        "api" => Protocol::Api,
        "api-ssl" => Protocol::ApiSsl,
        _ => usage(),
    }
}

/// Print usage and exit.
fn usage() -> ! {
    eprintln!("usage: raw_call <host> <api|api-ssl> <username> <password> <command> [attribute[=value] ...]");
    process::exit(2);
}

/// Example entry point.
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = Client::connect(ClientBuilder::new(
        args.host,
        args.protocol,
        Credentials {
            username: args.username,
            password: Some(args.password),
        },
    ))
    .await?;
    let attributes = args
        .attributes
        .iter()
        .map(|attribute| match attribute.split_once('=') {
            Some((key, value)) => (key, Some(value)),
            None => (attribute.as_str(), None),
        })
        .collect::<Vec<_>>();
    let rows = client.call(&args.command, &attributes).await?;
    println!("rows: {}", rows.len());
    for row in rows {
        println!("{:#?}", redact_command_row(&args.command, &row));
    }
    Ok(())
}
