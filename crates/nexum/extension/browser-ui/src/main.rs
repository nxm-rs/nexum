use browser_ui::App;
use leptos::*;
use tracing::trace;

fn main() {
    // print pretty errors in wasm https://github.com/rustwasm/console_error_panic_hook
    // This is not needed for tracing_wasm to work, but it is a common tool for getting proper error line numbers for panics.
    console_error_panic_hook::set_once();

    // Add this line:
    wasm_tracing::set_as_global_default();

    trace!("Starting the app");

    // Mount the view to the body
    mount_to_body(|| view! { <App /> });
}
