use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetOffers, prefix = "/api", endpoint = "get_offers")]
pub async fn get_offers() -> Result<Vec<OfferListItem>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_offers().await
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
}

#[server(name = GetOffer, prefix = "/api", endpoint = "get_offer")]
pub async fn get_offer(id: i64) -> Result<Offer, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_offer(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = SaveOffer, prefix = "/api", endpoint = "save_offer")]
pub async fn save_offer(offer: Offer) -> Result<Offer, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.save_offer(offer).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = offer;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = CommitOffer, prefix = "/api", endpoint = "commit_offer")]
pub async fn commit_offer(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.commit_offer(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = DeleteOffer, prefix = "/api", endpoint = "delete_offer")]
pub async fn delete_offer(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.delete_offer(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetOfferRevisions, prefix = "/api", endpoint = "get_offer_revisions")]
pub async fn get_offer_revisions(offer_id: i64) -> Result<Vec<shared::OfferRevision>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_offer_revisions(offer_id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = offer_id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = CreateOfferRevision, prefix = "/api", endpoint = "create_offer_revision")]
pub async fn create_offer_revision(offer_id: i64) -> Result<Offer, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.create_offer_revision(offer_id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = offer_id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}
