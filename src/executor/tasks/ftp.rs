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
    let cmd = match direction {
        TransferDirection::Download => format!(
            "curl -u {}:{} {} -o {}",
            shell_escape(username),
            shell_escape(password),
            shell_escape(&url),
            shell_escape(local_path)
        ),
        TransferDirection::Upload => format!(
            "curl -u {}:{} -T {} {}",
            shell_escape(username),
            shell_escape(password),
            shell_escape(local_path),
            shell_escape(&url)
        ),
    };
    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
}
