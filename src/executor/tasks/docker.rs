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

    // Write Dockerfile to temp dir using Rust to avoid all shell quoting issues
    let tmp = std::env::temp_dir().join(format!("kf-docker-{}", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::create_dir_all(&tmp) {
        return CommandResult {
            status: ExecutionStatus::Failed,
            exit_code: None,
            stdout: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
            stderr: CapturedOutput {
                text: format!("failed to create temp dir: {e}"),
                truncated: false,
            },
        };
    }
    if let Err(e) = std::fs::write(tmp.join("Dockerfile"), &dockerfile_content) {
        return CommandResult {
            status: ExecutionStatus::Failed,
            exit_code: None,
            stdout: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
            stderr: CapturedOutput {
                text: format!("failed to write Dockerfile: {e}"),
                truncated: false,
            },
        };
    }
    let tmp_path = tmp.display().to_string();
    let escaped_build_args = build_args
        .unwrap_or("")
        .split_whitespace()
        .map(|arg| shell_escape(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let mut cmd = format!(
        "docker build --progress=plain -t {} {} -f {}/Dockerfile {}",
        shell_escape(tag),
        escaped_build_args,
        shell_escape(&tmp_path),
        shell_escape(&tmp_path),
    );

    if run_after_build {
        cmd.push_str(&format!(" && docker run --rm {}", shell_escape(tag)));
    }

    cmd.push_str(&format!(
        " ; RET=$?; rm -rf {}; exit $RET",
        shell_escape(&tmp_path)
    ));

    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
}
