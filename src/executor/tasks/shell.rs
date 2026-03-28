use tokio::sync::oneshot;

use super::super::{CommandResult, run_command};

pub async fn run_shell_task(
    command: &str,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    run_command(command, run_as, timeout_secs, cancel_rx).await
}
