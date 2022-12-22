#![feature(is_some_and)]
#![warn(clippy::pedantic)]
#![allow(clippy::unused_async, clippy::module_name_repetitions)]

mod config;
mod router;
mod routes;
mod state;

use crate::{router::Router, state::AppState};
use anyhow::Result;
use axum::Server;
use chrono::Utc;
use config::Config;
use directories::ProjectDirs;
use std::{net::SocketAddr, path::Path, sync::Arc};
use tokio::{
    fs::{create_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
};
use tracing_subscriber::{filter, prelude::*};

pub fn error(message: &str) -> ! {
    tracing::error!(message);
    tracing::error!("Exiting due to previous error...");

    std::process::exit(-1);
}

async fn create_initial_config_file(config_path: &Path) -> Result<()> {
    let config_file_path = config_path.join("config.json");

    let default_config = serde_json::to_string_pretty(&Config::default())?;
    let config_file = File::create(config_file_path).await?;
    let mut bufwriter = BufWriter::new(config_file);

    _ = bufwriter.write(default_config.as_bytes()).await?;
    bufwriter.flush().await?;

    tracing::warn!(
        "A default configuration has been generated, please fill in the fields marked \
         with \"PLEASE CHANGE\" before running again."
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

    let data_dir_path = config_dir.data_dir();
    create_dir_all(data_dir_path).await?;

    let datetime = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);

    let debug_file = File::create(data_dir_path.join(format!("debug-{datetime}.log")))
        .await?
        .into_std()
        .await;

    let debug_log = tracing_subscriber::fmt::layer().with_writer(Arc::new(debug_file));

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

fn signal_handler() {
    tracing::info!("Received termination signal, shutting down gracefully...");

    // TODO: Handle graceful shutdown

    std::process::exit(0);
}

#[tokio::main]
async fn main() -> Result<()> {
    ctrlc::set_handler(signal_handler)?;

    init_logs().await?;
    let config = init_config().await?;

    let mut state = AppState::from_config(&config);
    state.connect().await?;

    let addr = SocketAddr::from((config.get_host(), config.get_port()));

    let router = Router::new(state);

    tracing::info!("Starting on {}...", addr);

    Server::bind(&addr).serve(router.build()).await?;

    Ok(())
}
