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
	web-ext build -s crates/nexum/extension/dist -a .

clippy:
	cargo clippy --all-targets --all-features --workspace -- -Dwarnings

build:
	cargo build --all-targets --all-features --workspace

test:
	cargo test --all-targets --all-features --workspace

# clippy-extension:
# 	cargo clippy --all-targets --all-features --target wasm32-unknown-unknown -p browser-ui -p worker -p injected -p injector -- -Dwarnings
