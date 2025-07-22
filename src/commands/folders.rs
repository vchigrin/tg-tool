use super::login::make_client_from_session_file;
use crate::serialization;
use eyre::Result;
use grammers_tl_types as tl_types;

use std::fs;
use std::path;

pub async fn handle_folders_backup_command(
    session_file: &path::Path,
    dst_file_path: &path::Path,
) -> Result<()> {
    let tg_client = make_client_from_session_file(session_file).await?;
    let request = tl_types::functions::messages::GetDialogFilters {};
    let tl_types::enums::messages::DialogFilters::Filters(filters) =
        tg_client.invoke(&request).await.unwrap();
    let f_out = fs::File::create(dst_file_path)?;
    let mut ser = serde_json::Serializer::new(f_out);
    serialization::DialogFiltersDef::serialize(&filters, &mut ser)?;
    Ok(())
}
