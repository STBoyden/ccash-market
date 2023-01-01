use crate::{
    commodity::{Commodity, CommodityUID},
    config::Config,
    offer::{Offer, OfferUID},
    user::{User, UserUID},
};
use anyhow::{Error, Result};
use ccash_rs::{methods as m, CCashSession, CCashUser};
use chrono::{DateTime, SecondsFormat, Utc};
use dashmap::{DashMap, DashSet};
use directories::ProjectDirs;
use flate2::{bufread::GzDecoder, write::GzEncoder, Compression};
use parking_lot::RwLock;
use rayon::prelude::{IntoParallelRefIterator, *};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, rename, File},
    io::{BufReader, BufWriter, Read},
    sync::Arc,
};
use uuid::Uuid;

pub type GState = Arc<RwLock<AppState>>;

#[derive(Serialize)]
pub struct AppProperties {
    ledger_host: String,
    market_username: String,
}

pub(crate) type Commodities = DashMap<CommodityUID, Arc<RwLock<Commodity>>>;
pub(crate) type Offers = DashMap<OfferUID, Arc<RwLock<Offer>>>;
pub(crate) type Users = DashMap<UserUID, Arc<RwLock<User>>>;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Data {
    pub(super) commodities: Commodities,
    pub(super) offers: Offers,
    pub(super) users: Users,
}

#[derive(Debug)]
pub struct AppState {
    ledger_host: String,
    ccash_session: Option<CCashSession>,
    market_user_uid: Option<UserUID>,
    market_user_details: (String, String),
    data: Data,
}

impl AppState {
    fn get_data() -> Data {
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
            .read(true)
            .create(true)
            .open(data_dir.join("data.gz"));

        if let Err(e) = file {
            tracing::error!("{e}");
            std::process::exit(-1);
        }
        let file = file.unwrap();

        let bufreader = BufReader::new(file);
        let mut decoder = GzDecoder::new(bufreader);
        let mut buffer = String::new();

        _ = decoder.read_to_string(&mut buffer);

        let data = serde_json::from_str::<Data>(&buffer);

        if let Ok(data) = data {
            data
        } else {
            tracing::warn!(
                "\"{}\" was empty or could not be read properly, using new data...",
                data_dir.join("data.gz").to_string_lossy()
            );
            Data::default()
        }
    }

    pub(crate) fn save_data(&self) -> Result<()> {
        let Some(project_dir) = ProjectDirs::from("", "", "ccash-market") else {
            let message = "Could not find suitable application directory!";

            tracing::error!(message);
            return Err(Error::msg(message));
        };

        let data_dir = project_dir.data_dir();

        if !data_dir.exists() {
            create_dir_all(data_dir)?;
        }

        let file_path = data_dir.join("data.gz");

        if file_path.exists() {
            let old_file = File::open(&file_path)?;
            let creation_time = DateTime::<Utc>::from(old_file.metadata()?.created()?)
                .to_rfc3339_opts(SecondsFormat::Secs, false);

            let file_name = format!("data-{creation_time}.gz.bak");

            tracing::info!(
                "Found previous \"data.gz\", backing up to \"data/{file_name}\"."
            );

            let backup_dir = data_dir.join("data");
            create_dir_all(&backup_dir)?;

            rename(&file_path, backup_dir.join(file_name))?;
        }

        let file = File::create(&file_path)?;

        tracing::info!("Writing data to {}...", file_path.to_string_lossy());

        let bufwriter = BufWriter::new(file);
        let mut encoder = GzEncoder::new(bufwriter, Compression::best());

        serde_json::to_writer(&mut encoder, &self.data)?;

        encoder.finish()?;

        tracing::info!("... Done!");

        Ok(())
    }

    pub(crate) fn from_config(config: &Config) -> Self {
        let ledger_host = if let Some(ledger_host) = config.get_ledger_host() {
            ledger_host
        } else {
            "Unset"
        };

        Self {
            ledger_host: ledger_host.to_owned(),
            ccash_session: None,
            market_user_uid: None,
            market_user_details: (
                config.get_market_username().to_owned(),
                config.get_market_password().to_owned(),
            ),
            data: Self::get_data(),
        }
    }

    pub(crate) async fn connect(&mut self) -> Result<()> {
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

        let contains = if let Ok(contains) = m::contains_user(session, &market_user).await
        {
            contains
        } else {
            false
        };

        if !contains {
            tracing::info!(
                "Market user with the name \"{}\" doesn't exist. Adding...",
                market_user.get_username()
            );

            if dbg!(m::add_user(session, &market_user).await)? {
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

        let market_user_id = if let Some((id, _)) = self
            .data
            .users
            .par_iter()
            .map(|rw| (*rw.key(), Arc::clone(rw.value())))
            .find_first(|(_, user)| {
                user.read().get_username() == market_user.get_username()
            }) {
            id
        } else {
            UserUID(Uuid::new_v4())
        };

        tracing::info!(
            "Market user \"{}\" has UUID of {market_user_id}",
            market_user.get_username()
        );

        if !self.data.users.contains_key(&market_user_id) {
            let market_user =
                Arc::new(RwLock::new(User::new(market_user.get_username())));

            self.data
                .users
                .insert(market_user_id, Arc::clone(&market_user));
        }

        self.market_user_uid = Some(market_user_id);

        Ok(())
    }

    fn get_market_user(&self) -> Option<Arc<RwLock<User>>> {
        let Some(uid) = self.market_user_uid  else {
            return None;
        };

        self.data.users.get(&uid).map(|user| Arc::clone(&*user))
    }

    pub fn get_or_add_user(&mut self, user: &CCashUser) -> UserUID {
        let mut iter = self.data.users.iter().filter(|kv| {
            let v = kv.value();
            v.read().get_username() == user.get_username()
        });

        if let Some(kv) = iter.next() {
            let uuid = kv.key();

            *uuid
        } else {
            let uuid = UserUID(Uuid::new_v4());
            let user = User::new(user.get_username());

            self.data.users.insert(uuid, Arc::new(RwLock::new(user)));

            uuid
        }
    }

    pub fn get_or_add_commodity(
        &mut self,
        commodity_name: &str,
        amount: u64,
        owner_id: UserUID,
    ) -> CommodityUID {
        let mut iter = self.data.commodities.iter().filter(|kv| {
            let v = kv.value();

            v.read().get_name() == commodity_name
        });

        if let Some(kv) = iter.next() {
            let uuid = kv.key();

            *uuid
        } else {
            let uuid = CommodityUID(Uuid::new_v4());
            let commodity = Commodity::new(
                commodity_name,
                Some(amount),
                Some({
                    let ds = DashSet::new();
                    ds.insert(owner_id);
                    ds
                }),
            );

            self.data
                .commodities
                .insert(uuid, Arc::new(RwLock::new(commodity)));

            uuid
        }
    }

    pub fn add_ask(
        &mut self,
        commodity_id: CommodityUID,
        user_id: UserUID,
        amount: u64,
        price_per_item: u64,
    ) -> OfferUID {
        let offer_id = OfferUID(Uuid::new_v4());
        let ask = Offer::Ask {
            user_id,
            commodity_id,
            datetime: Utc::now(),
            item_amount: amount,
            price_per_item,
        };

        self.data
            .offers
            .insert(offer_id, Arc::new(RwLock::new(ask)));

        offer_id
    }

    pub fn add_bid(
        &mut self,
        commodity_id: CommodityUID,
        user_id: UserUID,
        amount: u64,
        price_per_item: u64,
    ) -> OfferUID {
        let offer_id = OfferUID(Uuid::new_v4());
        let bid = Offer::Bid {
            user_id,
            commodity_id,
            datetime: Utc::now(),
            item_amount: amount,
            price_per_item,
        };

        self.data
            .offers
            .insert(offer_id, Arc::new(RwLock::new(bid)));

        offer_id
    }

    pub fn as_properties(&self) -> AppProperties {
        let market_username = if let Some(market_user) = self.get_market_user() {
            market_user.read().get_username().to_owned()
        } else {
            "Unknown".into()
        };

        AppProperties {
            ledger_host: self.ledger_host.clone(),
            market_username,
        }
    }

    pub(crate) fn get_offers(&self) -> &Offers { &self.data.offers }
    // pub(crate) fn get_offers_mut(&mut self) -> &mut Offers { &mut
    // self.data.offers }

    pub(crate) fn get_commodities(&self) -> &Commodities { &self.data.commodities }
    pub(crate) fn get_commodities_mut(&mut self) -> &mut Commodities {
        &mut self.data.commodities
    }

    pub(crate) fn get_users(&self) -> &Users { &self.data.users }
    pub(crate) fn get_users_mut(&mut self) -> &mut Users { &mut self.data.users }
}
