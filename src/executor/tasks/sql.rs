use tokio::sync::oneshot;

use crate::db::models::SqlDriver;

use super::super::{CommandResult, run_command, shell_escape};

pub async fn run_sql_task(
    driver: &SqlDriver,
    connection_string: &str,
    query: &str,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let cmd = match driver {
        SqlDriver::Postgres => format!(
            "psql {} -c {}",
            shell_escape(connection_string),
            shell_escape(query)
        ),
        SqlDriver::Mysql => format!(
            "mysql {} -e {}",
            shell_escape(connection_string),
            shell_escape(query)
        ),
        SqlDriver::Sqlite => format!(
            "sqlite3 {} {}",
            shell_escape(connection_string),
            shell_escape(query)
        ),
    };
    run_command(&cmd, run_as, timeout_secs, cancel_rx).await
}
