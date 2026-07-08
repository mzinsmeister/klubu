// Both helpers below are server-only, so their imports are too.
#[cfg(feature = "ssr")]
use leptos::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[cfg(feature = "ssr")]
pub async fn store_new_version(
    repo: &super::db::ActiveRepository,
    document_id: Option<i32>,
    extension: &str,
    media_type: &str,
    storage_key_prefix: &str,
    data: &[u8],
) -> Result<shared::Document, ServerFnError> {
    repo.store_new_version(document_id, extension, media_type, storage_key_prefix, data).await
}

#[cfg(feature = "ssr")]
pub async fn delete_document(
    repo: &super::db::ActiveRepository,
    document_id: i32,
) -> Result<(), ServerFnError> {
    repo.delete_document(document_id).await
}
