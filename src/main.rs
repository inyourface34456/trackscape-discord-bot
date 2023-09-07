mod commands;
mod database;
mod ge_api;
mod osrs_broadcast_extractor;

use anyhow::anyhow;
use mongodb::Database;
use serenity::async_trait;
use serenity::model::application::interaction::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::guild::Guild;
use serenity::model::id::GuildId;
use serenity::prelude::*;

use crate::database::BotMongoDb;
use crate::ge_api::ge_api::{get_item_mapping, GeItemMapping};
use crate::osrs_broadcast_extractor::osrs_broadcast_extractor::{extract_message, ClanMessage};
use shuttle_persist::PersistInstance;
use shuttle_secrets::SecretStore;
use tracing::info;

struct Bot {
    channel_to_check: u64,
    channel_to_send: u64,
    drop_price_threshold: u64,
    persist: PersistInstance,
    mongo_db: BotMongoDb,
}

#[async_trait]
impl EventHandler for Bot {
    async fn guild_create(&self, _ctx: Context, guild: Guild, is_new: bool) {
        info!("has been added to the guild {}", guild.name);
        if is_new {
            //This fires if it's a new guild it's been added to
            self.mongo_db.save_new_guild(guild.id.0).await;
            info!("Joined a new Discord Server: {}", guild.name);
        }
    }

    async fn guild_member_addition(
        &self,
        _ctx: Context,
        _new_member: serenity::model::guild::Member,
    ) {
        info!("New member added to guild {}", _new_member.user.name);
    }

    // async fn guild_delete(&self, _ctx: Context, _incomplete: serenity::model::guild::UnavailableGuild) {
    //     info!("We've been removed from a guild {}", _incomplete.id);
    // }
    async fn message(&self, ctx: Context, msg: Message) {
        // self.mongo_db.get_or_set_server();
        //in game chat channel
        if msg.channel_id == self.channel_to_check {
            info!("New message!\n");
            if msg.embeds.iter().count() > 0 {
                let author = msg.embeds[0].author.as_ref().unwrap().name.clone();
                let message = msg.embeds[0].description.as_ref().unwrap().clone();
                let clan_message = ClanMessage {
                    author,
                    message: message.clone(),
                };
                if clan_message.author == "Insomniacs" {
                    let item_mapping_from_state = self
                        .persist
                        .load::<GeItemMapping>("mapping")
                        .map_err(|e| info!("Saving Item Mapping Error: {e}"));
                    let possible_response =
                        extract_message(clan_message, item_mapping_from_state).await;
                    match possible_response {
                        None => {}
                        Some(response) => {
                            //Achievement Channel Id
                            info!("{}\n", message.clone());

                            if response.item_value.is_some() {
                                if response.item_value.unwrap() < self.drop_price_threshold as i64 {
                                    info!("The Item value is less than threshold, not sending message\n");
                                    return;
                                }
                            }

                            let channel = ctx.http.get_channel(self.channel_to_send).await.unwrap();
                            channel
                                .id()
                                .send_message(&ctx.http, |m| {
                                    m.embed(|e| {
                                        e.title(response.title)
                                            .description(response.message)
                                            .color(0x0000FF);
                                        match response.icon_url {
                                            None => {}
                                            Some(icon_url) => {
                                                e.image(icon_url);
                                            }
                                        }
                                        e
                                    })
                                })
                                .await
                                .unwrap();
                        }
                    }
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guild_id = GuildId(1148645741653393408);

        let commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands.create_application_command(|command| {
                commands::set_clan_chat_channel::register(command)
            })
        })
        .await;

        // println!("I now have the following guild slash commands: {:#?}", commands);

        //Use this for global commands
        // let guild_command = Command::create_global_application_command(&ctx.http, |command| {
        //     commands::wonderful_command::register(command)
        // })
        //     .await;
        //
        // println!("I created the following global slash command: {:#?}", guild_command);
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            // println!("Received command interaction: {:#?}", command);

            let content = match command.data.name.as_str() {
                "set_clan_chat_channel" => {
                    commands::set_clan_chat_channel::run(
                        &command.data.options,
                        &ctx,
                        &self.mongo_db,
                        command.guild_id.unwrap().0,
                    )
                    .await
                }
                _ => {
                    info!("not implemented :(");
                    None
                }
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| match content {
                    None => response.interaction_response_data(|message| {
                        message.content("Command Completed Successfully.")
                    }),
                    Some(reply) => {
                        response.interaction_response_data(|message| message.content(reply))
                    }
                })
                .await
            {
                println!("Cannot respond to slash command: {}", why);
            }
        }
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_persist::Persist] persist: PersistInstance,
    #[shuttle_shared_db::MongoDb] db: Database,
) -> shuttle_serenity::ShuttleSerenity {
    // Get the discord token set in `Secrets.toml`as
    let token = if let Some(token) = secret_store.get("DISCORD_TOKEN") {
        token
    } else {
        return Err(anyhow!("'DISCORD_TOKEN' was not found").into());
    };

    let in_game_channel = if let Some(token) = secret_store.get("IN_GAME_CHANNEL") {
        token
    } else {
        return Err(anyhow!("'IN_GAME_CHANNEL' was not found").into());
    };

    let channel_to_send_message_to =
        if let Some(token) = secret_store.get("CHANNEL_TO_SEND_MESSAGES_TO") {
            token
        } else {
            return Err(anyhow!("'CHANNEL_TO_SEND_MESSAGES_TO' was not found").into());
        };

    let drop_price_threshold = if let Some(token) = secret_store.get("DROP_PRICE_THRESHOLD") {
        token
    } else {
        return Err(anyhow!("'DROP_PRICE_THRESHOLD' was not found").into());
    };

    let ge_mapping_request = get_item_mapping().await;
    match ge_mapping_request {
        Ok(ge_mapping) => {
            let _state = persist
                .save::<GeItemMapping>("mapping", ge_mapping.clone())
                .map_err(|e| info!("Saving Item Mapping Error: {e}"));
        }
        Err(error) => {
            info!("Error getting ge mapping: {}", error)
        }
    }

    // Set gateway intents, which decides what events the bot will be notified about
    let intents =
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS;

    let client = Client::builder(&token, intents)
        .event_handler(Bot {
            channel_to_check: in_game_channel.parse::<u64>().unwrap(),
            channel_to_send: channel_to_send_message_to.parse::<u64>().unwrap(),
            drop_price_threshold: drop_price_threshold.parse::<u64>().unwrap(),
            persist,
            mongo_db: BotMongoDb::new(db),
        })
        .await
        .expect("Err creating client");

    Ok(client.into())
}
