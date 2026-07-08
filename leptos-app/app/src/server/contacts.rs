use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetContacts, prefix = "/api", endpoint = "get_contacts")]
pub async fn get_contacts() -> Result<Vec<Contact>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found in context"))?;
        repo.get_contacts().await
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
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

#[server(name = DeleteContact, prefix = "/api", endpoint = "delete_contact")]
pub async fn delete_contact(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.delete_contact(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}
