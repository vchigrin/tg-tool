use super::login::make_client_from_session_file;
use crate::utils;
use eyre::{eyre, Result};
use grammers_tl_types as tl_types;
use log::{error, info, warn};
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
    And(AssignConditionComposite),
    Or(AssignConditionComposite),
    Not(Box<AssignCondition>),
    TitleRegex(AssignConditionTitleRegex),
    InfoRegex(AssignConditionInfoRegex),
    DialogType(AssignConditionDialogType),
    ContactPresent(AssignConditionContactPresent),
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

#[derive(Deserialize)]
struct AssignConditionContactPresent {
    login: String,
}

#[derive(Deserialize, PartialEq)]
enum DialogType {
    User,
    Group,
    Channel,
}

#[derive(Deserialize)]
struct AssignConditionDialogType {
    dialog_type: DialogType,
}

#[derive(Deserialize)]
struct AssignConditionComposite {
    children: Vec<AssignCondition>,
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

    fn participants(&self) -> grammers_client::client::chats::ParticipantIter {
        self.tg_client.iter_participants(self.dialog.chat())
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

fn chat_title_match(regex_info: &AssignConditionTitleRegex, dialog_info: &DialogInfo) -> bool {
    regex_info
        .regex_match
        .is_match(dialog_info.dialog().chat().name())
}

async fn chat_info_match(regex_info: &AssignConditionInfoRegex, dialog_info: &DialogInfo) -> bool {
    let maybe_chat_full = match dialog_info.chat_full().await {
        Ok(chat) => chat,
        Err(e) => {
            error!(
                "Error {e:?} during ChatFullInfo fetching on dialog {:?}.",
                dialog_info.dialog()
            );
            return false;
        }
    };
    match maybe_chat_full {
        None => {
            // Most probably this is dialog with user, not chat.
            false
        }
        Some(chat_full) => regex_info.regex_match.is_match(get_about_string(chat_full)),
    }
}

async fn chat_contact_present(
    contact_info: &AssignConditionContactPresent,
    dialog_info: &DialogInfo,
) -> bool {
    let mut participants_iter = dialog_info.participants();
    loop {
        let participant_or_error = participants_iter.next().await;
        let maybe_participant = match participant_or_error {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    "Failed to enumerate participants on chat {} error {}",
                    dialog_info.dialog().chat().name(),
                    e
                );
                return false;
            }
        };
        if let Some(participant) = maybe_participant {
            if participant.user.username() == Some(&contact_info.login) {
                return true;
            }
        } else {
            break;
        }
    }
    false
}

fn chat_dialog_type_match(
    dialog_type_info: &AssignConditionDialogType,
    dialog_info: &DialogInfo,
) -> bool {
    match dialog_info.dialog().chat() {
        grammers_client::types::chat::Chat::User(_) => {
            dialog_type_info.dialog_type == DialogType::User
        }
        grammers_client::types::chat::Chat::Group(_) => {
            dialog_type_info.dialog_type == DialogType::Group
        }
        grammers_client::types::chat::Chat::Channel(_) => {
            dialog_type_info.dialog_type == DialogType::Channel
        }
    }
}

async fn chat_and_conditions(
    condition_info: &AssignConditionComposite,
    dialog_info: &DialogInfo,
) -> bool {
    for child in &condition_info.children {
        if !Box::pin(condition_match(child, dialog_info)).await {
            return false;
        }
    }
    true
}

async fn chat_or_conditions(
    condition_info: &AssignConditionComposite,
    dialog_info: &DialogInfo,
) -> bool {
    for child in &condition_info.children {
        if Box::pin(condition_match(child, dialog_info)).await {
            return true;
        }
    }
    false
}

async fn condition_match(condition: &AssignCondition, dialog_info: &DialogInfo) -> bool {
    match condition {
        AssignCondition::TitleRegex(condition_info) => {
            chat_title_match(condition_info, dialog_info)
        }
        AssignCondition::InfoRegex(condition_info) => {
            chat_info_match(condition_info, dialog_info).await
        }
        AssignCondition::ContactPresent(condition_info) => {
            chat_contact_present(condition_info, dialog_info).await
        }
        AssignCondition::DialogType(condition_info) => {
            chat_dialog_type_match(condition_info, dialog_info)
        }
        AssignCondition::And(condition_info) => {
            chat_and_conditions(condition_info, dialog_info).await
        }
        AssignCondition::Or(condition_info) => {
            chat_or_conditions(condition_info, dialog_info).await
        }
        AssignCondition::Not(child_condition) => {
            !Box::pin(condition_match(child_condition, dialog_info)).await
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
    let rules =
        ChatFilters::deserialize(&mut des).map_err(|e| eyre!("Failed parse rules file; {}", e))?;
    let tg_client = make_client_from_session_file(session_file).await?;
    let mut dialogs = tg_client.iter_dialogs();
    let mut filter_name_to_dialogs =
        collections::HashMap::<String, Vec<tl_types::enums::InputPeer>>::new();
    let mut dialog_infos = Vec::new();
    info!("Collecting dialogs");
    while let Some(dialog) = dialogs.next().await? {
        dialog_infos.push(DialogInfo::new(dialog, tg_client.clone()));
    }
    for (idx, dialog_info) in dialog_infos.iter().enumerate() {
        info!(
            "Processing dialog {} of {} ({})",
            idx + 1,
            dialog_infos.len(),
            dialog_info.dialog().chat.name()
        );
        if let Some(filter) = apply_rules(&rules, dialog_info).await {
            info!("Assigned to folder {}", filter.name);
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
