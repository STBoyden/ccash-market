mod config;

use std::{net::SocketAddr, path::Path};

use anyhow::Result;
use axum::{routing::get, Router, Server};
use config::Config;
use directories::ProjectDirs;
use tokio::{
    fs::{create_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
};

async fn create_initial_config(config_path: &Path) -> Result<()> {
    let config_file_path = config_path.join("config.json");

    if config_file_path.exists() {
        return Ok(());
    }

    let default_config = serde_json::to_string_pretty(&Config::default())?;
    let config_file = File::create(config_file_path).await?;
    let mut bufwriter = BufWriter::new(config_file);

    bufwriter.write(default_config.as_bytes()).await?;
    bufwriter.flush().await?;

    Ok(())
}

async fn read_config(config_path: &Path) -> Result<Config> {
    let config_file_path = config_path.join("config.json");

    if !config_file_path.exists() {
        todo!();
    }

    let config_file = File::open(config_file_path).await?;
    let mut bufreader = BufReader::new(config_file);
    let mut buffer = String::new();
    bufreader.read_to_string(&mut buffer).await?;

    if buffer.is_empty() {
        todo!();
    }

    Ok(serde_json::from_str::<Config>(&buffer)?)
}

async fn init() -> Result<Config> {
    let Some(config_dir) = ProjectDirs::from("uk.co", "STBoyden", "ccash-market") else {
        unimplemented!()
    };
    let config_dir = config_dir.config_dir();

    if !config_dir.exists() {
        tracing::debug!(
            "Config path not found: Creating {}...",
            config_dir.to_string_lossy().to_string()
        );

        create_dir_all(config_dir).await?;
        create_initial_config(config_dir).await?;
    }

    Ok(read_config(config_dir).await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = init().await?;

    let app = Router::new().route("/", get(root));
    let addr = SocketAddr::from((config.get_host(), config.get_port()));

    tracing::debug!("Starting on {}...", addr);

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

async fn root() -> &'static str { "Hello, world!" }
