//! `nexum-runtime` — host-side WebAssembly Component Model runtime for nexum modules.
//!
//! This binary loads a guest component that targets the universal
//! `web3:runtime` WIT package and invokes its `init` / `on-event`
//! exports. The host implements the imported interfaces
//! (`csn`, `local-store`, `remote-store`, `msg`, `logging`) as stubs for now,
//! providing a minimal but complete bootstrap of the runtime surface.
//!
//! The runtime is intentionally environment-agnostic: modules written
//! against `web3:runtime` can be executed in this host or in any other
//! conforming host (mobile, browser, embedded) without modification.

use std::time::Instant;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::error::Context as _;
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

// Wrap the macro-generated bindings in a private module so we can silence the
// strict workspace lints (`missing_docs`, `missing_debug_implementations`) for
// just the generated items without disabling them crate-wide.
#[allow(missing_docs, missing_debug_implementations)]
mod bindings {
    wasmtime::component::bindgen!({
        path: "../../../wit/web3-runtime",
        world: "headless-module",
        imports: { default: async },
        exports: { default: async },
    });
}

use bindings::{Config, HeadlessModule, web3};

/// Mutable host state shared across all WASI and `web3:runtime` host calls.
struct HostState {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl std::fmt::Debug for HostState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `WasiCtx` and `ResourceTable` do not themselves implement `Debug`,
        // so render a placeholder that satisfies the workspace lint without
        // exposing internal state.
        f.debug_struct("HostState").finish_non_exhaustive()
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

// -- Stub implementations for host interfaces --

impl web3::runtime::types::Host for HostState {}

impl web3::runtime::csn::Host for HostState {
    async fn request(
        &mut self,
        _chain_id: u64,
        method: String,
        _params: String,
    ) -> Result<String, web3::runtime::csn::JsonRpcError> {
        let start = Instant::now();
        eprintln!("[csn] request: {method}");
        let result = Err(web3::runtime::csn::JsonRpcError {
            code: -32601,
            message: format!("method not implemented: {method}"),
            data: None,
        });
        eprintln!("[timing] csn::request: {:?}", start.elapsed());
        result
    }
}

impl web3::runtime::local_store::Host for HostState {
    async fn get(&mut self, key: String) -> Result<Option<Vec<u8>>, String> {
        let start = Instant::now();
        eprintln!("[local-store] get: {key}");
        let result = Ok(None);
        eprintln!("[timing] local-store::get: {:?}", start.elapsed());
        result
    }

    async fn set(&mut self, key: String, _value: Vec<u8>) -> Result<(), String> {
        let start = Instant::now();
        eprintln!("[local-store] set: {key}");
        let result = Ok(());
        eprintln!("[timing] local-store::set: {:?}", start.elapsed());
        result
    }

    async fn delete(&mut self, key: String) -> Result<(), String> {
        let start = Instant::now();
        eprintln!("[local-store] delete: {key}");
        let result = Ok(());
        eprintln!("[timing] local-store::delete: {:?}", start.elapsed());
        result
    }

    async fn list_keys(&mut self, prefix: String) -> Result<Vec<String>, String> {
        let start = Instant::now();
        eprintln!("[local-store] list-keys: {prefix}");
        let result = Ok(vec![]);
        eprintln!("[timing] local-store::list-keys: {:?}", start.elapsed());
        result
    }
}

impl web3::runtime::remote_store::Host for HostState {
    async fn upload(
        &mut self,
        _data: Vec<u8>,
    ) -> Result<Vec<u8>, web3::runtime::remote_store::StoreError> {
        let start = Instant::now();
        let result = Err(web3::runtime::remote_store::StoreError {
            code: 501,
            message: "not implemented".into(),
        });
        eprintln!("[timing] remote-store::upload: {:?}", start.elapsed());
        result
    }

    async fn download(
        &mut self,
        _reference: Vec<u8>,
    ) -> Result<Vec<u8>, web3::runtime::remote_store::StoreError> {
        let start = Instant::now();
        let result = Err(web3::runtime::remote_store::StoreError {
            code: 501,
            message: "not implemented".into(),
        });
        eprintln!("[timing] remote-store::download: {:?}", start.elapsed());
        result
    }

    async fn feed_get(
        &mut self,
        _owner: Vec<u8>,
        _topic: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, web3::runtime::remote_store::StoreError> {
        let start = Instant::now();
        let result = Err(web3::runtime::remote_store::StoreError {
            code: 501,
            message: "not implemented".into(),
        });
        eprintln!("[timing] remote-store::feed-get: {:?}", start.elapsed());
        result
    }

    async fn feed_set(
        &mut self,
        _topic: Vec<u8>,
        _data: Vec<u8>,
    ) -> Result<Vec<u8>, web3::runtime::remote_store::StoreError> {
        let start = Instant::now();
        let result = Err(web3::runtime::remote_store::StoreError {
            code: 501,
            message: "not implemented".into(),
        });
        eprintln!("[timing] remote-store::feed-set: {:?}", start.elapsed());
        result
    }
}

impl web3::runtime::msg::Host for HostState {
    async fn publish(
        &mut self,
        content_topic: String,
        _payload: Vec<u8>,
    ) -> Result<(), web3::runtime::msg::MsgError> {
        let start = Instant::now();
        eprintln!("[msg] publish: {content_topic}");
        let result = Err(web3::runtime::msg::MsgError {
            code: 501,
            message: "not implemented".into(),
        });
        eprintln!("[timing] msg::publish: {:?}", start.elapsed());
        result
    }

    async fn query(
        &mut self,
        content_topic: String,
        _start_time: Option<u64>,
        _end_time: Option<u64>,
        _limit: Option<u32>,
    ) -> Result<Vec<web3::runtime::msg::Message>, web3::runtime::msg::MsgError> {
        let start = Instant::now();
        eprintln!("[msg] query: {content_topic}");
        let result = Ok(vec![]);
        eprintln!("[timing] msg::query: {:?}", start.elapsed());
        result
    }
}

impl web3::runtime::logging::Host for HostState {
    async fn log(&mut self, level: web3::runtime::logging::Level, message: String) {
        let start = Instant::now();
        let level_str = match level {
            web3::runtime::logging::Level::Trace => "TRACE",
            web3::runtime::logging::Level::Debug => "DEBUG",
            web3::runtime::logging::Level::Info => "INFO",
            web3::runtime::logging::Level::Warn => "WARN",
            web3::runtime::logging::Level::Error => "ERROR",
        };
        eprintln!("[{level_str}] {message}");
        eprintln!("[timing] logging::log: {:?}", start.elapsed());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let wasm_path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: nexum-runtime <path-to-component.wasm>"))?;

    println!("nexum-runtime: loading component from {wasm_path}");

    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;

    let start = Instant::now();
    let component =
        Component::from_file(&engine, &wasm_path).context("failed to load component")?;
    eprintln!("[timing] component load: {:?}", start.elapsed());

    let mut linker = Linker::<HostState>::new(&engine);
    HeadlessModule::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
        &mut linker,
        |state| state,
    )?;
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;

    let wasi = WasiCtxBuilder::new().inherit_stdio().build();

    let mut store = Store::new(
        &engine,
        HostState {
            wasi,
            table: ResourceTable::new(),
        },
    );

    let start = Instant::now();
    let bindings = HeadlessModule::instantiate_async(&mut store, &component, &linker)
        .await
        .context("failed to instantiate component")?;
    eprintln!("[timing] component instantiate: {:?}", start.elapsed());

    // Call init with config
    println!("nexum-runtime: calling init...");
    let config_entries: Config = vec![("name".into(), "example".into())];
    let start = Instant::now();
    match bindings.call_init(&mut store, &config_entries).await? {
        Ok(()) => println!("nexum-runtime: init succeeded"),
        Err(e) => println!("nexum-runtime: init failed: {e}"),
    }
    eprintln!("[timing] call_init: {:?}", start.elapsed());

    // Dispatch a test block event
    println!("nexum-runtime: dispatching test block event...");
    let block = web3::runtime::types::BlockData {
        chain_id: 1,
        number: 19_000_000,
        hash: vec![0xab; 32],
        timestamp: 1_700_000_000,
    };
    let event = web3::runtime::types::Event::Block(block);
    let start = Instant::now();
    match bindings.call_on_event(&mut store, &event).await? {
        Ok(()) => println!("nexum-runtime: on-event succeeded"),
        Err(e) => println!("nexum-runtime: on-event failed: {e}"),
    }
    eprintln!("[timing] call_on_event: {:?}", start.elapsed());

    println!("nexum-runtime: done");
    Ok(())
}
