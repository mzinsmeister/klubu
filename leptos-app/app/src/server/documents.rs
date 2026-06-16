use leptos::*;

#[cfg(feature = "ssr")]
pub async fn store_new_version(
    pool: &sqlx::PgPool,
    document_id: Option<i32>,
    extension: &str,
    media_type: &str,
    storage_key_prefix: &str,
    data: &[u8],
) -> Result<shared::Document, ServerFnError> {
    use sha2::{Sha256, Digest};
    use std::io::Write;

    // 1. Calculate SHA-256 checksum
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash_bytes = hasher.finalize();
    let checksum = hash_bytes.to_vec();

    // 2. Determine document ID and version
    let doc_id = match document_id {
        Some(id) => id,
        None => {
            // Create a new document
            let doc_row = sqlx::query!(
                "INSERT INTO document (media_type, extension, storage_key_prefix) VALUES ($1, $2, $3) RETURNING id",
                media_type,
                extension,
                storage_key_prefix
            )
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            doc_row.id
        }
    };

    // Get the latest version for this document
    let last_version = sqlx::query_scalar!(
        "SELECT MAX(version) FROM document_version WHERE document_id = $1",
        doc_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .flatten()
    .unwrap_or(0);

    let next_version = last_version + 1;

    // 3. Write file to filesystem
    let storage_dir = std::env::var("KLUBU_DOCUMENT_STORAGE_PATH")
        .unwrap_or_else(|_| "./document_storage".to_string());
    
    let file_name = format!("{}_{}.{}", storage_key_prefix, next_version, extension);
    let file_path = std::path::Path::new(&storage_dir).join(&file_name);

    // Ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ServerFnError::new(e.to_string()))?;
    }

    let mut file = std::fs::File::create(&file_path).map_err(|e| ServerFnError::new(e.to_string()))?;
    file.write_all(data).map_err(|e| ServerFnError::new(e.to_string()))?;

    // 4. Insert version in database
    sqlx::query!(
        "INSERT INTO document_version (document_id, version, checksum, is_tombstone) VALUES ($1, $2, $3, $4)",
        doc_id,
        next_version,
        checksum,
        0 // not a tombstone
    )
    .execute(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(shared::Document {
        id: doc_id as i64,
        media_type: media_type.to_string(),
        extension: extension.to_string(),
        storage_key_prefix: storage_key_prefix.to_string(),
    })
}

#[cfg(feature = "ssr")]
pub async fn delete_document(
    pool: &sqlx::PgPool,
    document_id: i32,
) -> Result<(), ServerFnError> {
    // Check if it's already tombstoned
    let last_tombstone = sqlx::query_scalar!(
        "SELECT is_tombstone FROM document_version WHERE document_id = $1 ORDER BY version DESC LIMIT 1",
        document_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    if last_tombstone == Some(1) {
        // Already deleted
        return Ok(());
    }

    let last_version = sqlx::query_scalar!(
        "SELECT MAX(version) FROM document_version WHERE document_id = $1",
        document_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .flatten()
    .unwrap_or(0);

    let next_version = last_version + 1;

    sqlx::query!(
        "INSERT INTO document_version (document_id, version, checksum, is_tombstone) VALUES ($1, $2, $3, $4)",
        document_id,
        next_version,
        &[] as &[u8], // empty checksum for tombstone
        1 // is tombstone
    )
    .execute(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
