be:
	wasm-pack build -t web --release -d ../dist/worker/ crates/extension/worker
	wasm-pack build -t web --release -d ../dist/injected/ crates/extension/injected
	wasm-pack build -t web --release -d ../dist/injector/ crates/extension/injector
	trunk build --release --config crates/extension/browser-ui/Trunk.toml index.html
	cp -r crates/extension/public/** crates/extension/dist/
