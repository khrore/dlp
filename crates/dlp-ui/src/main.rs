//! Binary entrypoint for the DLP UI crate.
#![expect(
    clippy::absolute_paths,
    reason = "Qualified standard-library paths are acceptable in this small entrypoint."
)]

use console_error_panic_hook as _;
use dlp_client as _;
use dlp_ui as _;
#[cfg(target_arch = "wasm32")]
use dlp_ui::App;
use leptos as _;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    use std::io::{self, Write as _};

    let mut stderr = io::stderr().lock();
    let _ignored = stderr.write_all(
        concat!(
            "`dlp-ui` is currently configured as a browser/WASM frontend. ",
            "Build it for `wasm32-unknown-unknown` and run it with a web host, ",
            "or add a Tauri/native entrypoint before using `cargo run -p dlp-ui`.\n"
        )
        .as_bytes(),
    );

    std::process::ExitCode::FAILURE
}
