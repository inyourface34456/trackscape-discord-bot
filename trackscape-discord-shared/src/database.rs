use crate::osrs_broadcast_extractor::osrs_broadcast_extractor::BroadcastType;
use mongodb::bson::doc;
use mongodb::{bson, Database};
use rand::Rng;
use regex::Error;
use serde::{Deserialize, Serialize};
use std::string::ToString;
use async_recursion::async_recursion;
use tracing::info;
use xxhash_rust::xxh3::Xxh3;
use crate::helpers::hash_string;

pub struct BotMongoDb {
    db: Database,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredGuild {
    pub guild_id: u64,
    //Channel to send broadcast messages
    pub clan_name: Option<String>,
    pub broadcast_channel: Option<u64>,
    //Channel to send clan chats messages
    pub clan_chat_channel: Option<u64>,
    pub drop_price_threshold: Option<i64>,
    pub allowed_broadcast_types: Option<Vec<BroadcastType>>,
    pub verification_code: String,
    pub hashed_verification_code: String,
    pub verified: bool,
}

impl RegisteredGuild {
    pub const COLLECTION_NAME: &'static str = "guilds";
    fn new(guild_id: u64) -> Self {
        let verification_code = Self::generate_code();
        let hashed_verification_code = hash_string(verification_code.clone());
        Self {
            guild_id,
            clan_name: None,
            broadcast_channel: None,
            clan_chat_channel: None,
            drop_price_threshold: None,
            allowed_broadcast_types: None,
            verification_code,
            hashed_verification_code,
            verified: false,
        }
    }

    fn generate_code() -> String {
        let mut code = String::new();
        let mut rng = rand::thread_rng();

        for i in 0..9 {
            code.push_str(&rng.gen_range(0..10).to_string());

            if i == 2 || i == 5 {
                code.push('-');
            }
        }
        code
    }


}

impl BotMongoDb {
    pub fn new(mongodb: Database) -> Self {
        Self { db: mongodb }
    }

    pub async fn save_new_guild(&self, guild_id: u64) {
        let mut guild = RegisteredGuild::new(guild_id);
        let check_for_unique_code = self
            .recursive_check_for_unique_code(guild.verification_code.clone())
            .await;
        match check_for_unique_code {
            Ok(code) => {
                guild.verification_code = code.clone();
                guild.hashed_verification_code = hash_string(code.clone());
                let collection = self.db.collection(RegisteredGuild::COLLECTION_NAME);
                collection
                    .insert_one(guild, None)
                    .await
                    .expect("Failed to insert document for a new guild.");
            }
            Err(_) => {}
        }
    }

    #[async_recursion]
    pub async fn recursive_check_for_unique_code(&self, code: String) -> Result<String, Error> {
        let collection = self
            .db
            .collection::<RegisteredGuild>(RegisteredGuild::COLLECTION_NAME);
        let hashed_code = hash_string(code.clone());
        let filter = doc! {"hashed_verification_code": hashed_code};
        let result = collection
            .find_one(filter, None)
            .await
            .expect("Was an error checking for unique code.");
        match result {
            Some(_) => {
                let new_code = RegisteredGuild::generate_code();
                self.recursive_check_for_unique_code(new_code).await
            }
            None => Ok(code),
        }
    }

    pub async fn get_guild_by_code_and_clan_name(
        &self,
        code: String,
        clan_name: String,
    ) -> Result<Option<RegisteredGuild>, Error> {
        let collection = self
            .db
            .collection::<RegisteredGuild>(RegisteredGuild::COLLECTION_NAME);
        let hashed_code = hash_string(code);
        let filter = doc! {"hashed_verification_code": hashed_code, "clan_name": clan_name};
        Ok(collection
            .find_one(filter, None)
            .await
            .expect("Failed to find document for the Discord guild."))
    }

    pub async fn get_guild_by_code(
        &self,
        code: String,
    ) -> Result<Option<RegisteredGuild>, Error> {
        let collection = self
            .db
            .collection::<RegisteredGuild>(RegisteredGuild::COLLECTION_NAME);
        let hashed_code = hash_string(code);
        let filter = doc! {"hashed_verification_code": hashed_code};
        Ok(collection
            .find_one(filter, None)
            .await
            .expect("Failed to find document for the Discord guild."))
    }

    pub async fn get_by_guild_id(&self, id: u64) -> Result<Option<RegisteredGuild>, mongodb::error::Error> {
        let collection = self
            .db
            .collection::<RegisteredGuild>(RegisteredGuild::COLLECTION_NAME);
        info!("Getting guild by id: {}", id);
        let filter = doc! { "guild_id": bson::to_bson(&id).unwrap()};
        let result = collection
            .find_one(filter.clone(), None)
            .await;
        info!("Result: {:?}", result);
        return match result {
            Ok(possible_guild) => {
                Ok(possible_guild)
            }
            Err(e) => {
                Err(e)
            }
        }

    }

    pub async fn update_guild(&self, guild: RegisteredGuild) {
        let collection = self.db.collection(RegisteredGuild::COLLECTION_NAME);
        let filter = doc! { "guild_id": bson::to_bson(&guild.guild_id).unwrap()};
        collection
            .replace_one(filter, guild, None)
            .await
            .expect("Failed to update document for the Discord guild.");
    }
}