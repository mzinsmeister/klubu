use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetContacts, prefix = "/api", endpoint = "get_contacts")]
pub async fn get_contacts(
    offset: u32,
    limit: u32,
    query: Option<String>,
) -> Result<Page<Contact>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found in context"))?;
        repo.get_contacts(offset, limit, query, false).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, query);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

/// The Archiv view: archived contacts, restorable. Same shape as
/// `get_contacts`, just the other side of the `archived_timestamp` filter.
#[server(name = GetArchivedContacts, prefix = "/api", endpoint = "get_archived_contacts")]
pub async fn get_archived_contacts(
    offset: u32,
    limit: u32,
    query: Option<String>,
) -> Result<Page<Contact>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found in context"))?;
        repo.get_contacts(offset, limit, query, true).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, query);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

/// Load every contact in bounded chunks for server-side matching/import paths.
/// Interactive lists stay paginated; this helper is reserved for operations
/// which must consider every possible supplier/customer match.
pub(crate) async fn get_all_contacts() -> Result<Vec<Contact>, ServerFnError> {
    const CHUNK_SIZE: u32 = 200;
    let mut contacts = Vec::new();

    loop {
        let page = get_contacts(contacts.len() as u32, CHUNK_SIZE, None).await?;
        let received = page.items.len();
        contacts.extend(page.items);
        if !page.has_more || received == 0 {
            return Ok(contacts);
        }
    }
}

#[server(name = SaveContact, prefix = "/api", endpoint = "save_contact")]
pub async fn save_contact(contact: Contact) -> Result<Contact, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.save_contact(contact).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = contact;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

/// Contacts are archived, never deleted: the id is the Kundennummer printed on
/// committed invoices and must stay resolvable.
#[server(name = ArchiveContact, prefix = "/api", endpoint = "archive_contact")]
pub async fn archive_contact(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.archive_contact(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = RestoreContact, prefix = "/api", endpoint = "restore_contact")]
pub async fn restore_contact(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.restore_contact(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}
