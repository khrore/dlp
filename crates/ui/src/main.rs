#[cfg(target_arch = "wasm32")]
use leptos::{mount::mount_to_body, prelude::*};
#[cfg(target_arch = "wasm32")]
use ui_app::App;

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    use std::io::Write;

    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(
        stderr,
        concat!(
            "`ui` is currently configured as a browser/WASM frontend. ",
            "Build it for `wasm32-unknown-unknown` and run it with a web host, ",
            "or add a Tauri/native entrypoint before using `cargo run -p ui`."
        )
    );

    std::process::ExitCode::FAILURE
}
