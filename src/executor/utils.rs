//! Utility functions: shell escaping, hex encoding, retry logic, and constants.

use crate::db::models::ExecutionStatus;

pub(crate) const DEFAULT_SCRIPT_TIMEOUT_SECS: u64 = 60;
pub(crate) const MAX_SCRIPT_OPERATIONS: u64 = 1_000_000;
pub(crate) const MAX_SCRIPT_STRING_SIZE: usize = 256 * 1024;

/// Maximum retry delay cap (1 hour).
const MAX_RETRY_DELAY_SECS: u64 = 3600;

pub(crate) fn shell_escape(s: &str) -> String {
    if cfg!(windows) {
        // Windows cmd.exe: wrap in double quotes, escape internal double quotes
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

pub(crate) fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let hex = hex.replace(' ', "");
    if !hex.len().is_multiple_of(2) {
        return Err("odd length".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("{e}")))
        .collect()
}

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Calculates the retry delay for the given attempt, capped at MAX_RETRY_DELAY_SECS.
pub(crate) fn calculate_retry_delay(delay_secs: u64, backoff: f64, attempt: u32) -> u64 {
    let delay = (delay_secs as f64) * backoff.powi((attempt - 1) as i32);
    // Clamp before cast to prevent f64 overflow → u64 saturation
    if delay.is_nan() || delay.is_infinite() || delay > MAX_RETRY_DELAY_SECS as f64 {
        MAX_RETRY_DELAY_SECS
    } else {
        (delay as u64).min(MAX_RETRY_DELAY_SECS)
    }
}

/// Returns true if the execution should be retried based on job config and status.
pub(crate) fn should_retry(retry_max: u32, status: ExecutionStatus, attempt_number: u32) -> bool {
    if retry_max == 0 {
        return false;
    }
    if attempt_number > retry_max {
        return false;
    }
    matches!(status, ExecutionStatus::Failed | ExecutionStatus::TimedOut)
}
