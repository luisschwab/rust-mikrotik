//! Generic print command execution.

use crate::client::Client;
use crate::commands::PrintCommand;
use crate::error::Result;

/// Run one print command and return the row count.
///
/// # Errors
///
/// Returns an error if the command cannot be sent, if `RouterOS` returns a trap
/// or fatal response, or if the connection closes before completion.
pub async fn run(client: &Client, command: PrintCommand) -> Result<usize> {
    client.call(command.as_path(), &[]).await.map(|rows| rows.len())
}
