# Contributing to Nexum

## Development environment

Components that you'd need installed:
1. [Rust toolchain](https://www.rust-lang.org/tools/install)
2. [`wasm-pack`](https://github.com/rustwasm/wasm-pack) (for packaging the wasm modules for the browser extension)
3. [`web-ext`](https://github.com/mozilla/web-ext) - optional utility that can make extension development more convenient

## Building

### TUI

To build the TUI:
```sh
$ cargo build -p tui
```

To run the TUI after building it:
```sh
$ ./target/debug/tui
```

To build and run the TUI at same time:
```sh
$ cargo run -p tui
```

### Extension

[`just`](https://just.systems/) recipes are provided for building, packaging
and running the browser extension.

To buil the extension, run:
```sh
$ just be
```

To run the extension using `web-ext`, run:
```sh
$ just re
```
