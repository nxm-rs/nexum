be:
	wasm-pack build -t web --release -d ../dist/worker/ crates/extension/worker
	wasm-pack build -t web --release -d ../dist/injected/ crates/extension/injected
	wasm-pack build -t web --release -d ../dist/injector/ crates/extension/injector
	mkdir -p crates/extension/browser-ui/public/style
	trunk build --release --config crates/extension/browser-ui/Trunk.toml index.html
	cp -r crates/extension/public/** crates/extension/dist/
	mkdir -p crates/extension/dist/browser-ui
	cp -r crates/extension/browser-ui/public/** crates/extension/dist/
	cp -r crates/extension/browser-ui/dist/* crates/extension/dist/browser-ui/
re:
	web-ext run -s crates/extension/dist
