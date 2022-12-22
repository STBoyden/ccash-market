use crate::{commodity::Commodity, config::Config};
use anyhow::{Error, Result};
use ccash_rs::{methods as m, CCashSession, CCashUser};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use directories::ProjectDirs;
use flate2::{bufread::GzDecoder, write::GzEncoder, Compression};
use parking_lot::RwLock;
use serde::Serialize;
use std::{
    fs::{create_dir_all, rename, File},
    io::{BufReader, BufWriter},
    sync::Arc,
};

pub type GState = Arc<RwLock<AppState>>;

#[derive(Serialize)]
pub struct AppProperties {
    ledger_host: String,
    market_username: String,
}

type Inventories = DashMap<String, Vec<Commodity>>;

#[derive(Clone, Debug)]
pub struct AppState {
    ledger_host: String,
    ccash_session: Option<CCashSession>,
    market_user: Option<CCashUser>,
    market_user_details: (String, String),
    inventories: Inventories,
}

impl AppState {
    fn get_inventories() -> Inventories {
        let Some(project_dir) = ProjectDirs::from("", "", "ccash-market") else {
          tracing::error!("Could not find suitable application directory!");
          std::process::exit(-1);
        };

        let data_dir = project_dir.data_dir();

        if !data_dir.exists() {
            if let Err(e) = create_dir_all(data_dir) {
                tracing::error!("{e}");
                std::process::exit(-1);
            }
        }

        let file = File::options()
            .write(true)
            .create(true)
            .open(data_dir.join("inventories.gz"));

        if let Err(e) = file {
            tracing::error!("{e}");
            std::process::exit(-1);
        }
        let file = file.unwrap();

        let bufreader = BufReader::new(file);
        let decoder = GzDecoder::new(bufreader);

        let data = serde_json::from_reader::<_, Inventories>(decoder);

        if let Ok(data) = data {
            data
        } else {
            tracing::warn!(
                "\"{}\" was empty or could not be read properly, using new data...",
                data_dir.join("inventories.gz").to_string_lossy()
            );
            Inventories::new()
        }
    }

    pub fn save_inventories(&self) -> Result<()> {
        let Some(project_dir) = ProjectDirs::from("", "", "ccash-market") else {
            let message = "Could not find suitable application directory!";

            tracing::error!(message);
            return Err(Error::msg(message));
        };

        let data_dir = project_dir.data_dir();

        if !data_dir.exists() {
            create_dir_all(data_dir)?;
        }

        let file_path = data_dir.join("inventories.gz");

        if file_path.exists() {
            let old_file = File::open(&file_path)?;
            let creation_time = DateTime::<Utc>::from(old_file.metadata()?.created()?)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, false);

            let file_name = format!("inventories-{creation_time}.gz.bak");

            tracing::info!(
                "Found previous \"inventories.gz\", backing up to \
                 \"inventories/{file_name}\"."
            );

            let backup_dir = data_dir.join("inventories");
            create_dir_all(&backup_dir)?;

            rename(&file_path, backup_dir.join(file_name))?;
        }

        let file = File::create(&file_path)?;

        tracing::info!(
            "Writing inventory data to {}...",
            file_path.to_string_lossy()
        );

        let bufwriter = BufWriter::new(file);
        let encoder = GzEncoder::new(bufwriter, Compression::best());

        serde_json::to_writer(encoder, &self.inventories)?;

        tracing::info!("... Done!");

        Ok(())
    }

    pub fn from_config(config: &Config) -> Self {
        let ledger_host = if let Some(ledger_host) = config.get_ledger_host() {
            ledger_host
        } else {
            "Unset"
        };

        Self {
            ledger_host: ledger_host.to_owned(),
            ccash_session: None,
            market_user: None,
            market_user_details: (
                config.get_market_username().to_owned(),
                config.get_market_password().to_owned(),
            ),
            inventories: Self::get_inventories(),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let mut ccash_session = CCashSession::new(&self.ledger_host);
        ccash_session.establish_connection().await?;

        self.ccash_session = Some(ccash_session);
        self.init_market_user().await?;

        Ok(())
    }

    async fn init_market_user(&mut self) -> Result<()> {
        if !self
            .ccash_session
            .clone()
            .is_some_and(|session| session.is_connected())
        {
            let message = "CCash session invalid or not connection not established.";

            tracing::error!(message);
            return Err(Error::msg(message));
        }

        let session = self.ccash_session.as_ref().unwrap();

        let market_user =
            CCashUser::new(&self.market_user_details.0, &self.market_user_details.1)?;

        if !m::contains_user(session, &market_user).await? {
            tracing::info!(
                "Market user with the name \"{}\" doesn't exist. Adding...",
                market_user.get_username()
            );

            if m::add_user(session, &market_user).await? {
                tracing::info!(
                    "User with name \"{}\" added to CCash instance.",
                    market_user.get_username()
                );
            }
        } else if !m::verify_password(session, &market_user).await? {
            let message = format!(
                "Market account \"{}\" has the incorrect password!",
                market_user.get_username()
            );

            tracing::error!(message);
            return Err(Error::msg(message));
        }

        self.market_user = Some(market_user);

        Ok(())
    }

    pub fn as_properties(&self) -> AppProperties {
        let market_username = if self.market_user.is_some() {
            self.market_user.as_ref().unwrap().get_username()
        } else {
            "Unknown"
        };

        AppProperties {
            ledger_host: self.ledger_host.clone(),
            market_username: market_username.to_owned(),
        }
    }
}
