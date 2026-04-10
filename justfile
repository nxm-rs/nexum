alias be := build-ext
build-ext:
	wasm-pack build -t web --release -d ../dist/worker/ crates/nexum/extension/worker
	wasm-pack build -t web --release -d ../dist/injected/ crates/nexum/extension/injected
	wasm-pack build -t web --release -d ../dist/injector/ crates/nexum/extension/injector
	mkdir -p crates/nexum/extension/browser-ui/public/style
	trunk build --release --config crates/nexum/extension/browser-ui/Trunk.toml index.html
	cp -r crates/nexum/extension/public/** crates/nexum/extension/dist/
	mkdir -p crates/nexum/extension/dist/browser-ui
	cp -r crates/nexum/extension/browser-ui/public/** crates/nexum/extension/dist/
	cp -r crates/nexum/extension/browser-ui/dist/* crates/nexum/extension/dist/browser-ui/

alias re := run-ext
run-ext:
	web-ext run -s crates/nexum/extension/dist

alias pe := pack-ext
pack-ext:
	web-ext build -s crates/nexum/extension/dist -a . --overwrite-dest

clippy:
	cargo clippy --all-targets --all-features --workspace -- -Dwarnings

build:
	cargo build --all-targets --all-features --workspace

test:
	cargo test --all-targets --all-features --workspace

# Build the WASM Component Model runtime host crate
build-runtime:
	cargo build -p nexum-runtime --release

# Build the example guest WASM module (wasm32-wasip2 target)
build-runtime-example:
	cargo build --target wasm32-wasip2 --release -p nexum-runtime-example

# Run the runtime host against the example guest module
run-runtime: build-runtime build-runtime-example
	cargo run -p nexum-runtime -- target/wasm32-wasip2/release/nexum_runtime_example.wasm

# Clippy gate for the runtime crate and example module
check-runtime:
	cargo clippy -p nexum-runtime --all-targets -- -Dwarnings
	cargo clippy --target wasm32-wasip2 -p nexum-runtime-example -- -Dwarnings

# clippy-extension:
# 	cargo clippy --all-targets --all-features --target wasm32-unknown-unknown -p browser-ui -p worker -p injected -p injector -- -Dwarnings
