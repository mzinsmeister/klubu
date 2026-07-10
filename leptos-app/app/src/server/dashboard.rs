use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetDashboardStats, prefix = "/api", endpoint = "get_dashboard_stats")]
pub async fn get_dashboard_stats() -> Result<DashboardStats, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_dashboard_stats().await
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
}
