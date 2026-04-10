// The `wit_bindgen::generate!` macro produces bindings that do not satisfy
// some of the stricter workspace lints (missing docs / debug impls on
// generated items, generated host-call shims with many arguments). These
// allows are scoped to this cdylib guest crate.
#![allow(
    missing_docs,
    missing_debug_implementations,
    unreachable_pub,
    clippy::too_many_arguments
)]
//! Example guest WASM module for the nexum runtime.
//!
//! Demonstrates the minimum scaffolding required to build a headless
//! module against the `web3:runtime` WIT package: implement the
//! `Guest` trait (covering `init` and `on_event`) and export it via
//! `wit_bindgen::export!`.
//!
//! Build with:
//!
//! ```sh
//! cargo build --target wasm32-wasip2 --release -p nexum-runtime-example
//! ```

wit_bindgen::generate!({
    path: "../../wit/web3-runtime",
    world: "headless-module",
});

use web3::runtime::logging;
use web3::runtime::types;

/// Example module implementation.
struct ExampleModule;

impl Guest for ExampleModule {
    /// Initialise the module with host-provided configuration.
    fn init(config: Vec<(String, String)>) -> Result<(), String> {
        let name = config
            .iter()
            .find(|(k, _)| k == "name")
            .map(|(_, v)| v.as_str())
            .unwrap_or("unknown");
        logging::log(
            logging::Level::Info,
            &format!("example module init (name={name})"),
        );
        Ok(())
    }

    /// Handle a runtime event dispatched by the host.
    fn on_event(event: types::Event) -> Result<(), String> {
        match &event {
            types::Event::Block(block) => {
                logging::log(
                    logging::Level::Info,
                    &format!(
                        "block {} on chain {} (ts={})",
                        block.number, block.chain_id, block.timestamp
                    ),
                );
            }
            types::Event::Logs(logs) => {
                logging::log(
                    logging::Level::Info,
                    &format!("received {} log entries", logs.len()),
                );
            }
            types::Event::Timer(ts) => {
                logging::log(logging::Level::Info, &format!("timer fired at {ts}"));
            }
            types::Event::Message(msg) => {
                logging::log(
                    logging::Level::Info,
                    &format!("message on topic {}", msg.content_topic),
                );
            }
        }
        Ok(())
    }
}

export!(ExampleModule);
