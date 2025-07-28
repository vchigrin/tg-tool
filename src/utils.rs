use eyre::Result;
use grammers_tl_types as tl_types;

pub async fn get_dialog_filters(
    tg_client: &grammers_client::Client,
) -> Result<tl_types::types::messages::DialogFilters> {
    let request = tl_types::functions::messages::GetDialogFilters {};
    let tl_types::enums::messages::DialogFilters::Filters(filters) =
        tg_client.invoke(&request).await?;
    Ok(filters)
}

pub fn get_filter_id(filter: &tl_types::enums::DialogFilter) -> Option<i32> {
    match filter {
        tl_types::enums::DialogFilter::Filter(filter) => Some(filter.id),
        tl_types::enums::DialogFilter::Default => None,
        tl_types::enums::DialogFilter::Chatlist(chat_list) => Some(chat_list.id),
    }
}

pub fn set_filter_id(filter: &mut tl_types::enums::DialogFilter, id: i32) {
    match filter {
        tl_types::enums::DialogFilter::Filter(filter) => {
            filter.id = id;
        }
        tl_types::enums::DialogFilter::Default => {}
        tl_types::enums::DialogFilter::Chatlist(chat_list) => {
            chat_list.id = id;
        }
    }
}

fn get_filter_title(filter: &tl_types::enums::DialogFilter) -> Option<&str> {
    match filter {
        tl_types::enums::DialogFilter::Filter(filter) => Some(&filter.title),
        tl_types::enums::DialogFilter::Default => None,
        tl_types::enums::DialogFilter::Chatlist(chat_list) => Some(&chat_list.title),
    }
}

fn find_matching_filter<'a>(
    current_filters: &'a [tl_types::enums::DialogFilter],
    saved_filter: &tl_types::enums::DialogFilter,
) -> Option<&'a tl_types::enums::DialogFilter> {
    // Telegram reuses IDs after folder re-creation, so it looks like is better
    // to use folder name as identifier for restoring
    let saved_title = get_filter_title(saved_filter);
    current_filters
        .iter()
        .find(|filter| get_filter_title(filter) == saved_title)
}

fn find_next_available_filter_id(filters: &[tl_types::enums::DialogFilter]) -> i32 {
    const MIN_CUSTOM_FILTER_ID: i32 = 2;
    let biggest_id = filters.iter().map(get_filter_id).max().flatten();
    match biggest_id {
        None => MIN_CUSTOM_FILTER_ID,
        Some(id) => id + 1,
    }
}

fn merge_peer_lists(
    updated: &mut Vec<tl_types::enums::InputPeer>,
    new_items: &Vec<tl_types::enums::InputPeer>,
) {
    for item in new_items {
        if updated.contains(item) {
            updated.push(item.clone());
        }
    }
}

fn merge_two_chat_lists(
    first: &tl_types::types::DialogFilterChatlist,
    second: &tl_types::types::DialogFilterChatlist,
) -> tl_types::enums::DialogFilter {
    let mut result = first.clone();
    result.has_my_invites |= second.has_my_invites;
    result.emoticon = result.emoticon.or(second.emoticon.clone());
    result.color = result.color.or(second.color);
    merge_peer_lists(&mut result.include_peers, &second.include_peers);
    merge_peer_lists(&mut result.pinned_peers, &second.pinned_peers);
    tl_types::enums::DialogFilter::Chatlist(result)
}

fn merge_two_filters(
    first: &tl_types::types::DialogFilter,
    second: &tl_types::types::DialogFilter,
) -> tl_types::enums::DialogFilter {
    let mut result = first.clone();
    result.contacts |= second.contacts;
    result.non_contacts |= second.non_contacts;
    result.groups |= second.groups;
    result.broadcasts |= second.broadcasts;
    result.bots |= second.bots;
    result.exclude_muted |= second.exclude_muted;
    result.exclude_read |= second.exclude_read;
    result.exclude_archived |= second.exclude_archived;
    result.emoticon = result.emoticon.or(second.emoticon.clone());
    result.color = result.color.or(second.color);
    merge_peer_lists(&mut result.include_peers, &second.include_peers);
    merge_peer_lists(&mut result.pinned_peers, &second.pinned_peers);
    merge_peer_lists(&mut result.exclude_peers, &second.exclude_peers);
    tl_types::enums::DialogFilter::Filter(result)
}

fn merge_chat_list_with_filter(
    first: &tl_types::types::DialogFilter,
    second: &tl_types::types::DialogFilterChatlist,
) -> tl_types::enums::DialogFilter {
    // DialogFilter contains more fields, so looks like better to produce
    // DialogFilter as result of such merge operation.
    let mut result = first.clone();
    result.emoticon = result.emoticon.or(second.emoticon.clone());
    result.color = result.color.or(second.color);
    merge_peer_lists(&mut result.include_peers, &second.include_peers);
    merge_peer_lists(&mut result.pinned_peers, &second.pinned_peers);
    tl_types::enums::DialogFilter::Filter(result)
}

fn merge_filters(
    current_filter: &tl_types::enums::DialogFilter,
    saved_filter: &tl_types::enums::DialogFilter,
) -> tl_types::enums::DialogFilter {
    match current_filter {
        // "Default" folders are not processed.
        tl_types::enums::DialogFilter::Default => tl_types::enums::DialogFilter::Default,
        tl_types::enums::DialogFilter::Filter(current_df) => match saved_filter {
            tl_types::enums::DialogFilter::Default => tl_types::enums::DialogFilter::Default,
            tl_types::enums::DialogFilter::Filter(saved_df) => {
                merge_two_filters(current_df, saved_df)
            }
            tl_types::enums::DialogFilter::Chatlist(saved_cl) => {
                merge_chat_list_with_filter(current_df, saved_cl)
            }
        },
        tl_types::enums::DialogFilter::Chatlist(current_cl) => match saved_filter {
            tl_types::enums::DialogFilter::Default => tl_types::enums::DialogFilter::Default,
            tl_types::enums::DialogFilter::Filter(saved_df) => {
                merge_chat_list_with_filter(saved_df, current_cl)
            }
            tl_types::enums::DialogFilter::Chatlist(saved_cl) => {
                merge_two_chat_lists(current_cl, saved_cl)
            }
        },
    }
}

/// Updates filters in the Telegram  based on desired_filters.
/// If some filters with same names already present, attempts merge their
/// contents.
/// During merging uses filter |title| field.
/// Filter |id| field in |desired_filters| is ignored.
pub async fn apply_dialog_filters(
    tg_client: &grammers_client::Client,
    desired_filters: &tl_types::types::messages::DialogFilters,
) -> Result<()> {
    let current_filters = get_dialog_filters(tg_client).await?;
    let mut next_available_filter_id = find_next_available_filter_id(&current_filters.filters);
    let mut maybe_error = None;
    for saved_filter in &desired_filters.filters {
        if let Some(current_filter) = find_matching_filter(&current_filters.filters, saved_filter) {
            // Matching filter found in current Telegram state,
            // update it rather then creating new.
            let current_filter_id = match get_filter_id(current_filter) {
                Some(id) => id,
                // ID is absend on "Default" filter - currently no action required
                // to merge it, so just skip.
                None => {
                    continue;
                }
            };
            let mut merged_filter = merge_filters(current_filter, saved_filter);
            set_filter_id(&mut merged_filter, current_filter_id);
            let update_request = tl_types::functions::messages::UpdateDialogFilter {
                id: current_filter_id,
                filter: Some(merged_filter),
            };
            println!("Sending update {update_request:?}");
            match tg_client.invoke(&update_request).await {
                Ok(_) => {}
                Err(error) => {
                    println!("Error during updating filter");
                    maybe_error = Some(error);
                }
            }
        } else {
            let new_filter_id = next_available_filter_id;
            next_available_filter_id += 1;
            let create_request = tl_types::functions::messages::UpdateDialogFilter {
                id: new_filter_id,
                filter: Some(saved_filter.clone()),
            };
            println!("Sending create {create_request:?}");
            match tg_client.invoke(&create_request).await {
                Ok(_) => {}
                Err(error) => {
                    println!("Error during creating filter");
                    maybe_error = Some(error);
                }
            }
        }
    }
    match maybe_error {
        None => Ok(()),
        Some(e) => Err(e.into()),
    }
}
