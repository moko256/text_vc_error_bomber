use std::sync::Arc;

use crate::model::Model;

use async_trait::async_trait;
use serenity::{
    model::{
        prelude::{
            command::{Command, CommandOptionType},
            component::ButtonStyle,
            interaction::{Interaction, InteractionResponseType},
            Activity, Channel, ChannelId, Guild, GuildChannel, GuildId, Message, Ready, UserId,
        },
        voice::VoiceState,
    },
    prelude::{Context, EventHandler, GatewayIntents, RwLock, TypeMapKey},
    Client,
};

pub async fn run_discord_app() {
    // Login with a bot token from the environment
    let token = std::env::var("DISCORD_TOKEN").expect("token");
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MESSAGES;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .await
        .unwrap();

    client
        .data
        .write()
        .await
        .insert::<DiscordData>(Arc::new(RwLock::new(Model::new())));

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        log::error!("An error occurred while running the client: {:?}", why);
    }
}

struct DiscordData;
impl TypeMapKey for DiscordData {
    type Value = Arc<RwLock<Model<GuildId, ChannelId, UserId>>>;
}

const ACTION_CMD_STATUS: &str = "status";
const ACTION_ID_BUTTON_OK: &str = "button_ok";
const ACTION_ID_BUTTON_NG: &str = "button_ng";

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        let data_read = ctx.data.read().await.get::<DiscordData>().unwrap().clone();
        let mut data = data_read.write().await;

        data.clear();

        if let Err(why) = Command::create_global_application_command(ctx.clone().http, |cmd| {
            cmd.name("tvb")
                .description("Command for text_vc_error_bomber.")
                .create_option(|opt| {
                    opt.name(ACTION_CMD_STATUS)
                        .kind(CommandOptionType::SubCommand)
                        .description("Print the innter states. For debug.")
                })
        })
        .await
        {
            log::warn!("Failed to register the slash commands: {:?}", why);
        }

        ctx.set_activity(Activity::watching("VCに入った人の行動"))
            .await;

        log::info!("Ready.");
    }

    async fn guild_create(&self, ctx: Context, guild: Guild) {
        let data_read = ctx.data.read().await.get::<DiscordData>().unwrap().clone();
        let mut data = data_read.write().await;

        for (channel_id, channel) in guild.channels {
            if let Channel::Guild(channel) = channel {
                data.add_channel_name_pair(channel.guild_id, channel_id, channel.name.clone());
            }
        }

        for (user_id, voice_state) in guild.voice_states {
            if let Some(channel_id) = voice_state.channel_id {
                data.add_or_update_user_voice_status(user_id, guild.id, channel_id);
            }
        }
    }

    async fn channel_create(&self, ctx: Context, channel: &GuildChannel) {
        let data_read = ctx.data.read().await.get::<DiscordData>().unwrap().clone();
        let mut data = data_read.write().await;

        data.add_channel_name_pair(channel.guild_id, channel.id, channel.name.clone());
    }

    async fn channel_update(&self, ctx: Context, new_data: Channel) {
        if let Channel::Guild(channel) = new_data {
            let data_read = ctx.data.read().await.get::<DiscordData>().unwrap().clone();
            let mut data = data_read.write().await;

            data.add_channel_name_pair(channel.guild_id, channel.id, channel.name);
        }
    }

    async fn channel_delete(&self, ctx: Context, channel: &GuildChannel) {
        let data_read = ctx.data.read().await.get::<DiscordData>().unwrap().clone();
        let mut data = data_read.write().await;

        data.remove_channel_name_pair(channel.guild_id, channel.id);
    }

    async fn voice_state_update(&self, ctx: Context, event: VoiceState) {
        let data_read = ctx.data.read().await.get::<DiscordData>().unwrap().clone();
        let mut data = data_read.write().await;

        if let (Some(guild_id), Some(channel_id)) = (event.guild_id, event.channel_id) {
            data.add_or_update_user_voice_status(event.user_id, guild_id, channel_id);
        } else {
            data.remove_user_voice_status(&event.user_id);
        }
    }

    async fn message(&self, ctx: Context, new_message: Message) {
        let data_read = ctx.data.read().await.get::<DiscordData>().unwrap().clone();
        let data = data_read.read().await;

        if let Some(guild_id) = new_message.guild_id {
            if data.msg_is_out_of_vc(&new_message.author.id, guild_id, new_message.channel_id) {
                if let Err(why) = new_message
                    .channel_id
                    .send_message(ctx.http, |msg| {
                        msg.reference_message(&new_message)
                            .content("警告：VC用テキストチャットじゃない所に誤爆していませんか？")
                            .allowed_mentions(|a| {
                                a.replied_user(true)
                                    .parse(serenity::builder::ParseValue::Users)
                            })
                            .components(|c| {
                                c.create_action_row(|row| {
                                    row.create_button(|b| {
                                        b.label("問題ない (警告削除)")
                                            .custom_id(ACTION_ID_BUTTON_OK)
                                            .style(ButtonStyle::Secondary)
                                    })
                                    .create_button(|b| {
                                        b.label("闇に葬る (警告・投稿削除)")
                                            .custom_id(ACTION_ID_BUTTON_NG)
                                            .style(ButtonStyle::Danger)
                                    })
                                })
                            })
                    })
                    .await
                {
                    log::warn!("Failed to post a warning: {:?}", why);
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let data_read = ctx.data.read().await.get::<DiscordData>().unwrap().clone();
        let data = data_read.read().await;

        match interaction {
            Interaction::ApplicationCommand(command) => {
                if command.data.options[0].name == ACTION_CMD_STATUS {
                    let mut buf = String::new();
                    buf.push_str("channel_names:\nguild_id,channel_id,name");
                    for ((guild_id, ch_id), name) in &data.channel_names {
                        buf.push_str(&format!("{},{},{}\n", guild_id.0, ch_id.0, name));
                    }
                    buf.push_str("user_vc_pairs:\nuser_id,guild_id,channel_id");
                    for (user_id, (guild_id, ch_id)) in &data.user_vc_pairs {
                        buf.push_str(&format!("{},{},{}\n", user_id.0, guild_id.0, ch_id.0));
                    }

                    log::info!("{}", buf);

                    let msg = if buf.chars().count() <= 2000 {
                        &buf
                    } else {
                        "内容が2000 Unicode Code Pointを越えました。ログを参照してください。"
                    };

                    if let Err(why) = command
                        .create_interaction_response(ctx.http, |b| {
                            b.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|d|
                                    d.content(msg)
                                        .flags(serenity::model::application::interaction::MessageFlags::EPHEMERAL)
                            )
                        })
                    .await {
                        log::warn!("Failed to post a status: {:?}", why);
                    }
                }
            }
            Interaction::MessageComponent(command) => match command.data.custom_id.as_str() {
                ACTION_ID_BUTTON_OK => {
                    if let Err(why) = command
                        .message
                        .channel_id
                        .delete_message(ctx.http, command.message.id)
                        .await
                    {
                        log::warn!("Failed to proceed OK actions: {:?}", why);
                    }
                }
                ACTION_ID_BUTTON_NG => {
                    if command
                        .message
                        .mentions
                        .iter()
                        .map(|f| f.id)
                        .any(|f| f == command.user.id)
                    {
                        if let Err(why) = command
                            .message
                            .channel_id
                            .delete_messages(
                                ctx.http,
                                [
                                    command.message.id,
                                    command
                                        .message
                                        .message_reference
                                        .unwrap()
                                        .message_id
                                        .unwrap(),
                                ],
                            )
                            .await
                        {
                            log::warn!("Failed to proceed NG actions: {:?}", why);
                        }
                    } else {
                        if let Err(why) = command
                        .create_interaction_response(ctx.http, |b| {
                            b.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|d|
                                    d.content("この操作は投稿者だけ可能です")
                                        .flags(serenity::model::application::interaction::MessageFlags::EPHEMERAL)
                                )
                        })
                        .await{
                            log::warn!("Failed to post a warning of actions: {:?}", why);
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
    }
}
