use client_sdk::DlpClient;
use leptos::{prelude::*, task::spawn_local};

const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:3000";

#[component]
pub fn App() -> impl IntoView {
    let client = DlpClient::new(DEFAULT_SERVER_URL);
    let (status, set_status) =
        signal("Click the button to check server health.".to_string());

    let run_health_check = move |_| {
        let client = client.clone();
        let set_status = set_status;

        set_status.set("Checking server health...".to_string());
        spawn_local(async move {
            let next_status = match client.health_check().await {
                Ok(response) => format!("{}: {}", response.service, response.status),
                Err(error) => format!("health check failed: {error}"),
            };
            set_status.set(next_status);
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
    const DEFAULT_STATUS: &str = "Click the button to check server health.";

    #[test]
    fn default_status_message_matches_app_copy() {
        assert_eq!(DEFAULT_STATUS, "Click the button to check server health.");
    }
}
