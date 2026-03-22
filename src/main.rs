mod api;
mod config;
mod cron_parser;
mod dag;
mod db;
mod error;
mod executor;
mod models;
mod scheduler;

use config::Config;
use dag::DagResolver;
use db::Db;
use executor::Executor;
use scheduler::Scheduler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kronforce=debug,info".parse().unwrap()),
        )
        .init();

    let config = Config::from_env();

    tracing::info!("opening database: {}", config.db_path);
    let db = Db::open(&config.db_path)?;
    db.migrate()?;

    let (scheduler_tx, scheduler_rx) = tokio::sync::mpsc::channel(64);

    let executor = Executor::new(db.clone());
    let dag = DagResolver::new(db.clone());
    let scheduler = Scheduler::new(db.clone(), executor, dag.clone(), scheduler_rx, &config);

    tokio::spawn(scheduler.run());

    let state = api::AppState {
        db,
        dag,
        scheduler_tx,
    };
    let app = api::router(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
