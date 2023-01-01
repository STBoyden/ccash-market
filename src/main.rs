#![feature(is_some_and, let_chains)]
#![warn(clippy::pedantic)]
#![allow(clippy::unused_async, clippy::module_name_repetitions)]

mod commodity;
mod config;
mod offer;
mod router;
mod routes;
mod state;
mod user;

use crate::{router::Router, state::AppState};
use anyhow::Result;
use axum::Server;
use chrono::Utc;
use config::Config;
use directories::ProjectDirs;
use parking_lot::RwLock;
use state::GState;
use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};
use tokio::{
    fs::{create_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    runtime::Handle,
    time::{interval_at, Instant},
};
use tracing_subscriber::{filter, prelude::*};

pub(crate) fn error(message: &str) -> ! {
    tracing::error!(message);
    tracing::error!("Exiting due to previous error...");

    std::process::exit(-1);
}

async fn create_initial_config_file(config_path: &Path) -> Result<()> {
    let config_file_path = config_path.join("config.json");

    let default_config = serde_json::to_string_pretty(&Config::default())?;
    let config_file = File::create(&config_file_path).await?;
    let mut bufwriter = BufWriter::new(config_file);

    _ = bufwriter.write(default_config.as_bytes()).await?;
    bufwriter.flush().await?;

    tracing::warn!(
        "A default configuration has been generated, please fill in the fields marked \
         with \"PLEASE CHANGE\" before running again."
    );
    tracing::warn!(
        "Configuration file location: {}",
        config_file_path.to_string_lossy()
    );

    Ok(())
}

async fn read_config(config_path: &Path) -> Result<Config> {
    let config_file_path = config_path.join("config.json");

    let config_file = File::open(config_file_path).await?;
    let mut bufreader = BufReader::new(config_file);
    let mut buffer = String::new();
    bufreader.read_to_string(&mut buffer).await?;

    let config = serde_json::from_str::<Config>(&buffer);

    if let Ok(config) = config {
        if config.get_ledger_host().is_none() {
            tracing::error!(
                "ledger_host not set in config.json! Please set to valid base URL."
            );
            tracing::error!("Exiting due to previous error...");

            std::process::exit(-1);
        }

        Ok(config)
    } else {
        Ok(Config::default())
    }
}

async fn init_logs() -> Result<()> {
    let Some(config_dir) = ProjectDirs::from("", "", "ccash-market") else {
        error("Could not find valid directory for project files.");
    };

    let stdout_log = tracing_subscriber::fmt::layer().compact();

    let data_dir_path = config_dir.data_dir().join("logs");
    create_dir_all(&data_dir_path).await?;

    let datetime = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);

    let debug_file = File::create(data_dir_path.join(format!("debug-{datetime}.log")))
        .await?
        .into_std()
        .await;

    let debug_log = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(false)
        .with_writer(Arc::new(debug_file));

    tracing_subscriber::registry()
        .with(
            stdout_log
                .with_filter(filter::LevelFilter::INFO)
                .and_then(debug_log),
        )
        .init();

    Ok(())
}

async fn init_config() -> Result<Config> {
    let Some(config_dir) = ProjectDirs::from("", "", "ccash-market") else {
        error("Could not find valid directory for project files.");
    };
    let config_dir = config_dir.config_dir();

    if !config_dir.exists() {
        tracing::warn!(
            "Config path not found: Creating {}...",
            config_dir.to_string_lossy().to_string()
        );

        create_dir_all(config_dir).await?;
        create_initial_config_file(config_dir).await?;
    }

    let mut config = read_config(config_dir).await?;
    let max_iterations: i32 = 3;
    let mut iterations = 0;

    'a: loop {
        if config != Config::default() {
            break 'a;
        } else if iterations >= max_iterations {
            error(&format!(
                "Couldn't get non-default configuration! Tried {iterations} time(s)"
            ));
        }

        create_initial_config_file(config_dir).await?;
        config = read_config(config_dir).await?;
        iterations += 1;
    }

    Ok(config)
}

fn signal_handler(state: &GState) {
    tracing::info!("Received termination signal, shutting down gracefully...");

    _ = state.read().save_data();

    std::process::exit(0);
}

async fn background_save(state: GState) {
    const MINUTE: u64 = 60;

    tracing::info!("Saving market data every 5 minutes.");

    let mut interval = interval_at(
        Instant::now() + Duration::from_secs(MINUTE),
        Duration::from_secs(5 * MINUTE),
    );

    loop {
        interval.tick().await;

        match state.read().save_data() {
            Ok(_) => tracing::info!("Market data saved!"),
            Err(e) => tracing::error!("Could not save market data: {e}"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logs().await?;
    let config = init_config().await?;

    let mut state = AppState::from_config(&config);
    state.connect().await?;

    let state_arc = Arc::new(RwLock::new(state));

    let tokio_handle = Handle::current();

    let state_arc_clone = state_arc.clone();
    tokio_handle.spawn(background_save(state_arc_clone));

    let state_arc_clone = state_arc.clone();
    ctrlc::set_handler(move || signal_handler(&state_arc_clone))?;

    let addr = SocketAddr::from((config.get_host(), config.get_port()));

    tracing::info!("Starting on http://{addr}...");

    let router = Router::new(state_arc, config.get_ledger_host());
    Server::bind(&addr).serve(router.build()).await?;

    Ok(())
}
