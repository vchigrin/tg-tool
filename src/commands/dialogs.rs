use super::login::make_client_from_session_file;
use crate::utils;
use eyre::Result;
use grammers_tl_types as tl_types;
use serde::Deserialize;
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

fn condition_match(condition: &AssignCondition, dialog: &grammers_client::types::Dialog) -> bool {
    match condition {
        AssignCondition::TitleRegex(regex_condition) => {
            regex_condition.regex_match.is_match(dialog.chat().name())
        }
    }
}

fn apply_rules<'a>(
    filters: &'a ChatFilters,
    dialog: &grammers_client::types::Dialog,
) -> Option<&'a ChatFilter> {
    filters
        .chat_filters
        .iter()
        .find(|filter| condition_match(&filter.condition, dialog))
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
    while let Some(dialog) = dialogs.next().await? {
        if let Some(filter) = apply_rules(&rules, &dialog) {
            let items: &mut Vec<tl_types::enums::InputPeer> =
                if let Some(v) = filter_name_to_dialogs.get_mut(&filter.name) {
                    v
                } else {
                    filter_name_to_dialogs.insert(filter.name.clone(), Vec::new());
                    filter_name_to_dialogs.get_mut(&filter.name).unwrap()
                };
            items.push(dialog.chat().pack().to_input_peer());
        }
    }
    assign_peers(&tg_client, &filter_name_to_dialogs).await?;
    Ok(())
}
