use crate::config::Config;
use anyhow::{Error, Result};
use ccash_rs::{methods as m, CCashSession, CCashUser};
use serde::Serialize;

#[derive(Serialize)]
pub struct AppProperties {
    ledger_host: String,
    market_username: String,
}

#[derive(Clone, Debug)]
pub struct AppState {
    ledger_host: String,
    ccash_session: Option<CCashSession>,
    market_user: Option<CCashUser>,
    market_user_details: (String, String),
}

impl AppState {
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
