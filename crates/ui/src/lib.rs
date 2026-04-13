//! Browser UI for the DLP control plane.
#![expect(
    clippy::absolute_paths,
    reason = "Leptos view macro expansion emits qualified paths."
)]
#![expect(
    clippy::same_name_method,
    reason = "The Leptos component macro generates a builder method."
)]
use console_error_panic_hook as _;
use dlp_client::{DlpClient, WorkersClientExt};
use leptos::{prelude::*, task::spawn_local};

const DEFAULT_STATUS: &str = "Click the button to check server health.";
const API_BASE_URL: &str = env!("DLP_UI_API_BASE_URL");

/// Renders the main application shell.
#[component]
pub fn App() -> impl IntoView {
    let health_client = DlpClient::new(API_BASE_URL);
    let (status, status_setter) = signal(DEFAULT_STATUS.to_owned());

    let run_health_check = move |_| {
        let request_client = health_client.clone();
        let response_setter = status_setter;

        response_setter.set("Checking server health...".to_owned());
        spawn_local(async move {
            let next_status = match request_client.health_check().await {
                Ok(response) => format!("{}: {}", response.service, response.status),
                Err(error) => format!("health check failed: {error}"),
            };
            response_setter.set(next_status);
        });
    };

    view! {
        <main>
            <h1>"DLP UI"</h1>
            <button on:click=run_health_check>"Health Check"</button>
            <p>{move || status.get()}</p>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::DEFAULT_STATUS;

    #[test]
    fn default_status_message_matches_app_copy() {
        assert_eq!(DEFAULT_STATUS, "Click the button to check server health.");
    }
}
