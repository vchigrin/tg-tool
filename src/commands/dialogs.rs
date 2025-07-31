use super::login::make_client_from_session_file;
use crate::utils;
use eyre::Result;
use grammers_tl_types as tl_types;
use serde::Deserialize;
use std::cell;
use std::collections;
use std::fs;
use std::path;

#[derive(Deserialize)]
struct ChatFilters {
    chat_filters: Vec<ChatFilter>,
}

#[derive(Deserialize)]
struct ChatFilter {
    name: String,
    condition: AssignCondition,
}

#[derive(Deserialize)]
enum AssignCondition {
    //  AndCondition(AssignConditionAnd),
    TitleRegex(AssignConditionTitleRegex),
    InfoRegex(AssignConditionInfoRegex),
    // DialogType(AssignConditionDialogType),
    // ContactPresent(AssignConditionContactPresent),
}

struct RegexDef {}

impl<'de> RegexDef {
    fn deserialize<D>(deserializer: D) -> Result<regex::Regex, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let content = String::deserialize(deserializer)?;
        let result = regex::Regex::new(&content).map_err(serde::de::Error::custom)?;
        Ok(result)
    }
}

#[derive(Deserialize)]
struct AssignConditionTitleRegex {
    #[serde(with = "RegexDef")]
    regex_match: regex::Regex,
}

#[derive(Deserialize)]
struct AssignConditionInfoRegex {
    #[serde(with = "RegexDef")]
    regex_match: regex::Regex,
}

struct DialogInfo {
    dialog: grammers_client::types::Dialog,
    tg_client: grammers_client::Client,
    chat_full: cell::OnceCell<tl_types::enums::ChatFull>,
}

impl DialogInfo {
    fn new(dialog: grammers_client::types::Dialog, tg_client: grammers_client::Client) -> Self {
        Self {
            dialog,
            tg_client,
            chat_full: cell::OnceCell::new(),
        }
    }

    fn dialog(&self) -> &grammers_client::types::Dialog {
        &self.dialog
    }

    async fn chat_full_impl(&self) -> Result<Option<grammers_tl_types::enums::ChatFull>> {
        let tl_types::enums::messages::ChatFull::Full(full) = match &self.dialog.chat {
            grammers_client::types::Chat::User(_) => {
                return Ok(None);
            }
            grammers_client::types::Chat::Group(group) => match &group.raw {
                tl_types::enums::Chat::Empty(_)
                | tl_types::enums::Chat::Forbidden(_)
                | tl_types::enums::Chat::ChannelForbidden(_) => {
                    return Ok(None);
                }
                tl_types::enums::Chat::Chat(chat) => {
                    self.tg_client
                        .invoke(&tl_types::functions::messages::GetFullChat { chat_id: chat.id })
                        .await?
                }
                tl_types::enums::Chat::Channel(channel) => {
                    let inp_channel =
                        tl_types::enums::InputChannel::Channel(tl_types::types::InputChannel {
                            channel_id: channel.id,
                            access_hash: channel.access_hash.unwrap(),
                        });
                    self.tg_client
                        .invoke(&tl_types::functions::channels::GetFullChannel {
                            channel: inp_channel,
                        })
                        .await?
                }
            },
            grammers_client::types::Chat::Channel(channel) => {
                let inp_channel =
                    tl_types::enums::InputChannel::Channel(tl_types::types::InputChannel {
                        channel_id: channel.id(),
                        access_hash: channel.raw.access_hash.unwrap(),
                    });
                self.tg_client
                    .invoke(&tl_types::functions::channels::GetFullChannel {
                        channel: inp_channel,
                    })
                    .await?
            }
        };
        Ok(Some(full.full_chat))
    }

    async fn chat_full(&self) -> Result<Option<&grammers_tl_types::enums::ChatFull>> {
        if let Some(val) = self.chat_full.get() {
            return Ok(Some(val));
        }
        if let Some(full) = self.chat_full_impl().await? {
            self.chat_full.set(full).unwrap();
        } else {
            return Ok(None);
        }
        Ok(Some(self.chat_full.get().unwrap()))
    }
}

fn get_about_string(chat_full: &tl_types::enums::ChatFull) -> &str {
    match chat_full {
        tl_types::enums::ChatFull::Full(full) => &full.about,
        tl_types::enums::ChatFull::ChannelFull(full) => &full.about,
    }
}

async fn condition_match(condition: &AssignCondition, dialog_info: &DialogInfo) -> bool {
    match condition {
        AssignCondition::TitleRegex(regex_condition) => regex_condition
            .regex_match
            .is_match(dialog_info.dialog().chat().name()),
        AssignCondition::InfoRegex(regex_condition) => {
            let maybe_chat_full = match dialog_info.chat_full().await {
                Ok(chat) => chat,
                Err(e) => {
                    println!(
                        "Error {e:?} during ChatFullInfo fetching on dialog {:?}.",
                        dialog_info.dialog()
                    );
                    return false;
                }
            };
            match maybe_chat_full {
                None => {
                    // Mot probably this is dialog with user, not chat.
                    false
                }
                Some(chat_full) => regex_condition
                    .regex_match
                    .is_match(get_about_string(chat_full)),
            }
        }
    }
}

async fn apply_rules<'a>(
    filters: &'a ChatFilters,
    dialog_info: &DialogInfo,
) -> Option<&'a ChatFilter> {
    for filter in &filters.chat_filters {
        if condition_match(&filter.condition, dialog_info).await {
            return Some(filter);
        }
    }
    None
}

async fn assign_peers(
    tg_client: &grammers_client::Client,
    filter_name_to_dialogs: &collections::HashMap<String, Vec<tl_types::enums::InputPeer>>,
) -> Result<()> {
    let mut desired_filters = grammers_tl_types::types::messages::DialogFilters {
        tags_enabled: false,
        filters: Vec::new(),
    };

    for (name, peers) in filter_name_to_dialogs {
        let new_item = grammers_tl_types::types::DialogFilterChatlist {
            has_my_invites: false,
            id: 0, // unused
            title: name.to_owned(),
            emoticon: None,
            color: None,
            pinned_peers: Vec::new(),
            include_peers: peers.clone(),
        };
        desired_filters
            .filters
            .push(grammers_tl_types::enums::DialogFilter::Chatlist(new_item));
    }
    utils::apply_dialog_filters(tg_client, &desired_filters).await
}

pub async fn handle_dialogs_assign_command(
    session_file: &path::Path,
    rules_file_path: &path::Path,
) -> Result<()> {
    let f_in = fs::File::open(rules_file_path)?;
    let mut des = serde_json::Deserializer::from_reader(f_in);
    let rules = ChatFilters::deserialize(&mut des)?;
    let tg_client = make_client_from_session_file(session_file).await?;
    let mut dialogs = tg_client.iter_dialogs();
    let mut filter_name_to_dialogs =
        collections::HashMap::<String, Vec<tl_types::enums::InputPeer>>::new();
    let mut dialog_infos = Vec::new();
    while let Some(dialog) = dialogs.next().await? {
        dialog_infos.push(DialogInfo::new(dialog, tg_client.clone()));
    }
    for dialog_info in &dialog_infos {
        if let Some(filter) = apply_rules(&rules, dialog_info).await {
            let items: &mut Vec<tl_types::enums::InputPeer> =
                if let Some(v) = filter_name_to_dialogs.get_mut(&filter.name) {
                    v
                } else {
                    filter_name_to_dialogs.insert(filter.name.clone(), Vec::new());
                    filter_name_to_dialogs.get_mut(&filter.name).unwrap()
                };
            items.push(dialog_info.dialog().chat().pack().to_input_peer());
        }
    }
    assign_peers(&tg_client, &filter_name_to_dialogs).await?;
    Ok(())
}
