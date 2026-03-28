use std::io::Write;

use tokio::sync::oneshot;

use crate::db::models::{FtpProtocol, TransferDirection};

use super::super::{CommandResult, run_command, shell_escape};

#[allow(clippy::too_many_arguments)]
pub async fn run_ftp_task(
    protocol: &FtpProtocol,
    host: &str,
    port: Option<u16>,
    username: &str,
    password: &str,
    direction: &TransferDirection,
    remote_path: &str,
    local_path: &str,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let port_part = port.map(|p| format!(":{}", p)).unwrap_or_default();
    let proto = match protocol {
        FtpProtocol::Ftp => "ftp",
        FtpProtocol::Ftps => "ftps",
        FtpProtocol::Sftp => "sftp",
    };
    let url = format!("{}://{}{}{}", proto, host, port_part, remote_path);

    // Write credentials to a temporary netrc file to avoid exposing them in process arguments
    let netrc_content = format!(
        "machine {}\nlogin {}\npassword {}\n",
        host, username, password
    );
    let netrc_file = match tempfile::NamedTempFile::new() {
        Ok(mut f) => {
            if let Err(e) = f.write_all(netrc_content.as_bytes()) {
                return CommandResult {
                    status: crate::db::models::ExecutionStatus::Failed,
                    exit_code: None,
                    stdout: super::super::CapturedOutput {
                        text: String::new(),
                        truncated: false,
                    },
                    stderr: super::super::CapturedOutput {
                        text: format!("failed to write netrc file: {e}"),
                        truncated: false,
                    },
                };
            }
            f
        }
        Err(e) => {
            return CommandResult {
                status: crate::db::models::ExecutionStatus::Failed,
                exit_code: None,
                stdout: super::super::CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: super::super::CapturedOutput {
                    text: format!("failed to create netrc file: {e}"),
                    truncated: false,
                },
            };
        }
    };
    let netrc_path = shell_escape(&netrc_file.path().to_string_lossy());

    let cmd = match direction {
        TransferDirection::Download => format!(
            "curl --netrc-file {} {} -o {}",
            netrc_path,
            shell_escape(&url),
            shell_escape(local_path)
        ),
        TransferDirection::Upload => format!(
            "curl --netrc-file {} -T {} {}",
            netrc_path,
            shell_escape(local_path),
            shell_escape(&url)
        ),
    };
    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
    // netrc_file is dropped here, automatically deleting the temp file
}
