use crate::db::models::ExecutionStatus;

use super::super::{CapturedOutput, CommandResult};

pub fn run_file_push_task(
    filename: &str,
    destination: &str,
    content_base64: &str,
    permissions: Option<&str>,
    overwrite: bool,
) -> CommandResult {
    use base64::Engine;
    let decoded = match base64::engine::general_purpose::STANDARD.decode(content_base64) {
        Ok(bytes) => bytes,
        Err(e) => {
            return CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: Some(1),
                stdout: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: format!("base64 decode error: {e}"),
                    truncated: false,
                },
            };
        }
    };

    let dest = std::path::Path::new(destination);

    // Check overwrite
    if !overwrite && dest.exists() {
        return CommandResult {
            status: ExecutionStatus::Failed,
            exit_code: Some(1),
            stdout: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
            stderr: CapturedOutput {
                text: format!("file already exists: {} (overwrite=false)", destination),
                truncated: false,
            },
        };
    }

    // Create parent dirs
    if let Some(parent) = dest.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return CommandResult {
            status: ExecutionStatus::Failed,
            exit_code: Some(1),
            stdout: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
            stderr: CapturedOutput {
                text: format!("failed to create directory {}: {e}", parent.display()),
                truncated: false,
            },
        };
    }

    // Write file
    let size = decoded.len();
    if let Err(e) = std::fs::write(dest, &decoded) {
        return CommandResult {
            status: ExecutionStatus::Failed,
            exit_code: Some(1),
            stdout: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
            stderr: CapturedOutput {
                text: format!("failed to write file: {e}"),
                truncated: false,
            },
        };
    }

    // Set permissions (Unix only)
    #[cfg(unix)]
    if let Some(perm_str) = permissions
        && let Ok(mode) = u32::from_str_radix(perm_str, 8)
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        let _ = std::fs::set_permissions(dest, perms);
    }

    CommandResult {
        status: ExecutionStatus::Succeeded,
        exit_code: Some(0),
        stdout: CapturedOutput {
            text: format!(
                "File '{}' written to {} ({} bytes)",
                filename, destination, size
            ),
            truncated: false,
        },
        stderr: CapturedOutput {
            text: String::new(),
            truncated: false,
        },
    }
}
