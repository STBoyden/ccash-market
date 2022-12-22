use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Config {
    host: Cow<'static, str>,
    port: u16,
    ledger_host: Option<String>,
    market_username: String,
    market_password: String,
}

impl Config {
    pub fn get_host(&self) -> [u8; 4] {
        let mut res = [0; 4];
        let splits = self.host.split('.');

        let vec = splits
            .filter_map(|num| num.parse::<u8>().ok())
            .collect::<Vec<_>>();

        vec.iter()
            .take(4)
            .enumerate()
            .for_each(|(index, x)| res[index] = *x);

        res
    }

    pub fn get_port(&self) -> u16 { self.port }

    #[allow(dead_code)]
    pub fn set_ledger_host(&mut self, base_url: &str) {
        self.ledger_host = Some(base_url.to_owned());
    }

    pub fn get_ledger_host(&self) -> Option<&String> { self.ledger_host.as_ref() }

    pub fn get_market_username(&self) -> &str { &self.market_username }
    pub fn get_market_password(&self) -> &str { &self.market_password }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3000,
            ledger_host: None,
            market_username: "market".into(),
            market_password: "PLEASE CHANGE".into(),
        }
    }
}
