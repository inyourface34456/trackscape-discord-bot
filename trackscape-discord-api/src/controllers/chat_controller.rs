use crate::cache::Cache;

use actix_web::{error, post, web, HttpRequest, Scope};
use serenity::builder::CreateMessage;
use serenity::http::Http;
use serenity::json;
use serenity::json::Value;
use shuttle_persist::PersistInstance;
use shuttle_runtime::tracing::info;
use trackscape_discord_shared::database::BotMongoDb;
use trackscape_discord_shared::ge_api::ge_api::GeItemMapping;
use trackscape_discord_shared::helpers::hash_string;
use trackscape_discord_shared::osrs_broadcast_extractor::osrs_broadcast_extractor::{
    extract_message, get_wiki_clan_rank_image_url, ClanMessage,
};

#[derive(Debug)]
struct MyError {
    message: &'static str,
}

#[post("/new-message")]
async fn hello_world(
    req: HttpRequest,
    discord_http_client: web::Data<Http>,
    cache: web::Data<Cache>,
    new_chat: web::Json<Vec<ClanMessage>>,
    mongodb: web::Data<BotMongoDb>,
    persist: web::Data<PersistInstance>,
) -> actix_web::Result<String> {
    let possible_verification_code = req.headers().get("verification-code");

    if let None = possible_verification_code {
        let result = Err(MyError {
            message: "No verification code was set",
        });
        return result.map_err(|err| error::ErrorBadRequest(err.message));
    }

    let verification_code = possible_verification_code.unwrap().to_str().unwrap();
    //checks to make sure the registered guild exists for the RuneScape clan
    let registered_guild_query = mongodb
        .get_guild_by_code(verification_code.to_string())
        .await;

    let registered_guild_successful_query = if let Ok(registered_guild) = registered_guild_query {
        registered_guild
    } else {
        registered_guild_query.map_err(|err| error::ErrorInternalServerError(err))?
    };

    let mut registered_guild = if let Some(registered_guild) = registered_guild_successful_query {
        registered_guild
    } else {
        let result = Err(MyError {
            message: "The verification code was not found",
        });
        return result.map_err(|err| error::ErrorBadRequest(err.message));
    };

    for chat in new_chat.clone() {
        //Checks to make sure the message has not already been process since multiple people could be submitting them
        let message_content_hash = hash_string(chat.message.clone());
        match cache.get_value(message_content_hash.clone()).await {
            Some(_) => continue,
            None => {
                cache
                    .set_value(message_content_hash.clone(), "true".to_string())
                    .await;
            }
        }

        if let None = registered_guild.clan_name {
            registered_guild.clan_name = Some(chat.clan_name.clone());
            mongodb.update_guild(registered_guild.clone()).await
        }

        match registered_guild.clan_chat_channel {
            Some(channel_id) => {
                let mut clan_chat_to_discord = CreateMessage::default();
                let author_image = match chat.clan_name.clone() == chat.sender.clone() {
                    true => {
                        "https://oldschool.runescape.wiki/images/Your_Clan_icon.png".to_string()
                    }
                    false => get_wiki_clan_rank_image_url(chat.rank.clone()),
                };

                clan_chat_to_discord.embed(|e| {
                    e.title("")
                        .author(|a| a.name(chat.sender.clone()).icon_url(author_image))
                        .description(chat.message.clone())
                        .color(0x0000FF)
                });
                let map = json::hashmap_to_json_map(clan_chat_to_discord.0);
                discord_http_client
                    .send_message(channel_id, &Value::from(map))
                    .await
                    .unwrap();
            }
            _ => {}
        }

        if let Some(broadcast_channel_id) = registered_guild.broadcast_channel {
            let item_mapping_from_state = persist
                .load::<GeItemMapping>("mapping")
                .map_err(|e| info!("Saving Item Mapping Error: {e}"));

            let possible_broadcast = extract_message(chat.clone(), item_mapping_from_state).await;
            match possible_broadcast {
                None => {}
                Some(broadcast) => {
                    info!("{}\n", chat.message.clone());

                    if broadcast.item_value.is_some() {
                        if let Some(drop_threshold) = registered_guild.drop_price_threshold {
                            if broadcast.item_value.unwrap() < drop_threshold {
                                //Item is above treshhold
                                //TODO i dont think this is working
                            }
                        }
                    }
                    let mut create_message = CreateMessage::default();
                    create_message.embed(|e| {
                        e.title(broadcast.title)
                            .description(broadcast.message)
                            .color(0x0000FF);
                        match broadcast.icon_url {
                            None => {}
                            Some(icon_url) => {
                                e.image(icon_url);
                            }
                        }
                        e
                    });

                    let map = json::hashmap_to_json_map(create_message.0);
                    discord_http_client
                        .send_message(broadcast_channel_id, &Value::from(map))
                        .await
                        .unwrap();
                }
            };
        }
    }

    return Ok("Message processed".to_string());
}

pub fn chat_controller() -> Scope {
    web::scope("/chat").service(hello_world)
}
