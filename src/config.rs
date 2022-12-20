use std::borrow::Cow;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    host: Cow<'static, str>,
    port: u16,
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3000,
        }
    }
}
