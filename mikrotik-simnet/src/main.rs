//! Command-line entry point for local `RouterOS` CHR simulation.

use std::process::ExitCode;

use argh::FromArgs;
use mikrotik_simnet::ChrArch;
use mikrotik_simnet::ROUTEROS_VERSIONS;
use mikrotik_simnet::RouterOsChannel;
use mikrotik_simnet::RunOptions;
use tracing::error;
use tracing_subscriber::EnvFilter;

/// Local `RouterOS` CHR simulation harness.
#[derive(Debug, FromArgs)]
struct Cli {
    /// subcommand to run
    #[argh(subcommand)]
    command: Command,
}

/// Simnet subcommands.
#[derive(Debug, FromArgs)]
#[argh(subcommand)]
enum Command {
    /// run one topology manifest
    Run(RunCommand),
    /// print a Mermaid diagram for one topology manifest
    Mermaid(MermaidCommand),
    /// list cataloged `RouterOS` versions and inferred CHR images
    ListVersions(ListVersionsCommand),
}

/// Run one topology manifest.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "run")]
struct RunCommand {
    /// stop router processes after checks pass instead of waiting for Ctrl-C
    #[argh(switch)]
    non_interactive: bool,
    /// topology TOML manifest path
    #[argh(positional)]
    topology: String,
}

/// Print a Mermaid diagram for one topology manifest.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "mermaid")]
struct MermaidCommand {
    /// topology TOML manifest path
    #[argh(positional)]
    topology: String,
}

/// List cataloged `RouterOS` versions and inferred CHR images.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "list-versions")]
struct ListVersionsCommand {}

#[tokio::main]
async fn main() -> ExitCode {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_level(true)
        .init();

    let cli = argh::from_env();

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Dispatch the requested subcommand.
async fn run(cli: Cli) -> mikrotik_simnet::Result<()> {
    match cli.command {
        Command::Run(command) => {
            let options = if command.non_interactive {
                RunOptions::non_interactive()
            } else {
                RunOptions::default()
            };
            mikrotik_simnet::run_topology_with_options(command.topology, options).await
        }
        Command::Mermaid(command) => {
            print!("{}", mikrotik_simnet::topology_mermaid(command.topology)?);
            Ok(())
        }
        Command::ListVersions(_) => list_versions(),
    }
}

/// Print the cataloged versions and the CHR image architecture inferred for this host.
fn list_versions() -> mikrotik_simnet::Result<()> {
    let plan = mikrotik_simnet::version_list()?;

    println!(
        "{:<10}  {:<18}  {:<10}  AVAILABLE_IMAGES",
        "VERSION", "CHANNELS", "GUEST_ARCH"
    );
    for image in plan.selected_images {
        let version = ROUTEROS_VERSIONS
            .iter()
            .find(|version| version.version == image.version)
            .expect("version list image should come from the catalog");
        println!(
            "{:<10}  {:<18}  {:<10}  {}",
            image.version,
            channel_list(version.channels),
            arch_name(image.guest_arch),
            version
                .image_arches
                .iter()
                .map(|arch| arch_name(*arch))
                .collect::<Vec<_>>()
                .join(",")
        );
    }

    Ok(())
}

/// Return a comma-separated channel list.
fn channel_list(channels: &[RouterOsChannel]) -> String {
    channels
        .iter()
        .map(|channel| match channel {
            RouterOsChannel::Stable => "stable",
            RouterOsChannel::LongTerm => "long-term",
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Return the CLI spelling for a CHR image architecture.
const fn arch_name(arch: ChrArch) -> &'static str {
    match arch {
        ChrArch::X86_64 => "x86_64",
        ChrArch::Aarch64 => "aarch64",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_subcommand() {
        let cli = Cli::from_args(&["mikrotik-simnet"], &["run", "topology.toml"]).expect("run subcommand should parse");

        let Command::Run(command) = cli.command else {
            panic!("run subcommand should parse");
        };
        assert_eq!(command.topology, "topology.toml");
        assert!(!command.non_interactive);
    }

    #[test]
    fn parses_run_non_interactive_switch() {
        let cli = Cli::from_args(&["mikrotik-simnet"], &["run", "--non-interactive", "topology.toml"])
            .expect("run subcommand should parse");

        let Command::Run(command) = cli.command else {
            panic!("run subcommand should parse");
        };
        assert_eq!(command.topology, "topology.toml");
        assert!(command.non_interactive);
    }

    #[test]
    fn parses_mermaid_subcommand() {
        let cli = Cli::from_args(&["mikrotik-simnet"], &["mermaid", "topology.toml"])
            .expect("mermaid subcommand should parse");

        let Command::Mermaid(command) = cli.command else {
            panic!("mermaid subcommand should parse");
        };
        assert_eq!(command.topology, "topology.toml");
    }

    #[test]
    fn parses_list_versions_subcommand() {
        assert!(matches!(
            Cli::from_args(&["mikrotik-simnet"], &["list-versions"])
                .expect("list-versions subcommand should parse")
                .command,
            Command::ListVersions(_)
        ));
    }

    #[test]
    fn formats_channel_list() {
        assert_eq!(channel_list(&[RouterOsChannel::Stable]), "stable");
        assert_eq!(
            channel_list(&[RouterOsChannel::Stable, RouterOsChannel::LongTerm]),
            "stable,long-term"
        );
    }

    #[test]
    fn arch_name_uses_cli_spelling() {
        assert_eq!(arch_name(ChrArch::X86_64), "x86_64");
        assert_eq!(arch_name(ChrArch::Aarch64), "aarch64");
    }
}
