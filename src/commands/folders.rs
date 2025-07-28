use super::login::make_client_from_session_file;
use crate::serialization;
use crate::utils;
use eyre::Result;
use grammers_tl_types as tl_types;

use std::fs;
use std::path;

pub async fn handle_folders_backup_command(
    session_file: &path::Path,
    dst_file_path: &path::Path,
) -> Result<()> {
    let tg_client = make_client_from_session_file(session_file).await?;
    let filters = utils::get_dialog_filters(&tg_client).await?;
    let f_out = fs::File::create(dst_file_path)?;
    let mut ser = serde_json::Serializer::new(f_out);
    serialization::DialogFiltersDef::serialize(&filters, &mut ser)?;
    Ok(())
}

pub async fn handle_folders_clear_command(session_file: &path::Path) -> Result<()> {
    let tg_client = make_client_from_session_file(session_file).await?;
    let filters = utils::get_dialog_filters(&tg_client).await?;
    for filter in &filters.filters {
        let maybe_filter_id = utils::get_filter_id(filter);
        if let Some(filter_id) = maybe_filter_id {
            let del_request = tl_types::functions::messages::UpdateDialogFilter {
                id: filter_id,
                filter: None,
            };
            tg_client.invoke(&del_request).await?;
        }
    }
    Ok(())
}

pub async fn handle_folders_restore_command(
    session_file: &path::Path,
    src_file_path: &path::Path,
) -> Result<()> {
    let f_in = fs::File::open(src_file_path)?;
    let mut deserializer = serde_json::Deserializer::from_reader(f_in);
    let saved_filters: tl_types::types::messages::DialogFilters =
        serialization::DialogFiltersDef::deserialize(&mut deserializer)?;
    let tg_client = make_client_from_session_file(session_file).await?;
    utils::apply_dialog_filters(&tg_client, &saved_filters).await?;
    Ok(())
}
