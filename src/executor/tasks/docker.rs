use tokio::sync::oneshot;

use super::super::{CapturedOutput, CommandResult, run_command, shell_escape};
use crate::db::models::ExecutionStatus;
use crate::executor::scripts::ScriptStore;

#[allow(clippy::too_many_arguments)]
pub async fn run_docker_build_task(
    script_name: &str,
    image_tag: Option<&str>,
    run_after_build: bool,
    build_args: Option<&str>,
    script_store: Option<&ScriptStore>,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let store = match script_store {
        Some(s) => s,
        None => {
            return CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: "script store not available".to_string(),
                    truncated: false,
                },
            };
        }
    };

    let dockerfile_content = match store.read_code(script_name) {
        Ok(code) => code,
        Err(e) => {
            return CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: format!("failed to read Dockerfile script '{}': {}", script_name, e),
                    truncated: false,
                },
            };
        }
    };

    let tag = image_tag.unwrap_or(script_name);

    // Write Dockerfile to a temp location and build
    let mut cmd = format!(
        "TMPDIR=$(mktemp -d) && echo {} > \"$TMPDIR/Dockerfile\" && docker build -t {} -f \"$TMPDIR/Dockerfile\" \"$TMPDIR\"",
        shell_escape(&dockerfile_content),
        shell_escape(tag)
    );

    if let Some(args) = build_args {
        cmd = format!(
            "TMPDIR=$(mktemp -d) && echo {} > \"$TMPDIR/Dockerfile\" && docker build -t {} {} -f \"$TMPDIR/Dockerfile\" \"$TMPDIR\"",
            shell_escape(&dockerfile_content),
            shell_escape(tag),
            args
        );
    }

    if run_after_build {
        cmd.push_str(&format!(" && docker run --rm {}", shell_escape(tag)));
    }

    cmd.push_str(" && rm -rf \"$TMPDIR\"");

    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
}
