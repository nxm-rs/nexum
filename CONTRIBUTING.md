# Contributing to Nexum

Thank you for your interest in contributing to Nexum! This document outlines the standards and expectations for contributions to ensure consistency and quality across the project.

## AI Assistance Disclosure

**If you are using any kind of AI assistance while contributing to Nexum (code generation, documentation writing, PR descriptions, review responses, etc.), this must be disclosed in the pull request.**

This disclosure requirement applies to all forms of AI assistance including but not limited to:
- Code generation or completion beyond trivial tab-completion of single keywords
- Documentation or comment generation
- Pull request descriptions and commit messages
- Responses to review comments

**Why we require disclosure:**

1. **Respect for reviewers**: Transparency about AI assistance allows maintainers to apply appropriate levels of scrutiny and helps set expectations for the review process.

2. **Code ownership**: You must thoroughly understand any code you submit, regardless of how it was generated. Be prepared to explain and defend implementation choices during review.

3. **Quality expectations**: AI-generated code often requires significant rework. Pull requests that require substantial changes due to quality issues may be closed.

## Pull Request Process

### Before Opening a PR

1. **Link to an accepted issue**: Pull requests should correspond to previously discussed and accepted issues or feature proposals. Unsolicited PRs may be closed or remain stale indefinitely.

2. **Use Discussions for design**: Pull requests are **not** the place to discuss feature design or architecture decisions. Use [matrix rooms](https://matrix.to/#/#nexum:nxm.rs) for brainstorming and design conversations. Once design is settled and an issue is marked as "accepted," implementation can begin.

3. **Self-review thoroughly**: Review your own changes before requesting review from others. Ensure your code:
   - Compiles without warnings (`cargo clippy` must pass)
   - Passes all existing tests
   - Includes tests for new functionality
   - Follows existing code patterns and style
   - Has appropriate documentation

### PR Description Requirements

Every pull request must include:

1. **Summary**: Clear description of what changes and why
2. **Testing**: How the changes were tested
3. **AI Assistance**: If applicable, disclose what AI tools were used and for what purpose
4. **Related Issues**: Link to the issue(s) being addressed

### After Opening a PR

- Be responsive to review feedback
- Be prepared to explain and defend your implementation choices
- Understand that maintainers may request significant changes or close PRs that don't meet quality standards

## Code Quality Standards

### Rust Code

- **Lints**: Code must pass `cargo clippy --all-targets --all-features --workspace -- -Dwarnings`
- **Tests**: All tests must pass via `cargo test --all-targets --all-features --workspace`
- **Documentation**: Public APIs must have doc comments
- **Error handling**: Use appropriate error types (avoid `.unwrap()` in library code)
- **Workspace lints**: Follow the workspace-level lints defined in `Cargo.toml`:
  - Warn on missing debug implementations and documentation
  - Deny unused must-use and rust-2018-idioms
  - Follow clippy warnings for code quality

### WASM Code

- **Size optimization**: WASM modules must be optimized for size (use `wasm-pack build --release`)
- **Error handling**: Errors must be properly handled and communicated across the JS boundary
- **Testing**: WASM code should include `wasm-bindgen-test` tests where applicable

### Commit Messages

- Use clear, descriptive commit messages
- Follow the conventional commit format: `<type>: <description>`
  - Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, etc.
- Reference issue numbers when applicable: `fix: resolve timeout issue (#42)`

## Testing Requirements

### Unit Tests

- New functionality must include unit tests
- Bug fixes should include regression tests
- Tests should be focused and test one thing

### Integration Tests

- Changes to the extension or RPC layer may require integration tests
- Test important user-facing workflows

### Running Tests

```sh
# Run all tests
cargo test --all-targets --all-features --workspace

# Run tests for a specific package
cargo test -p nexum-rpc

# Run a specific test
cargo test test_name

# Run WASM tests (requires wasm-pack)
wasm-pack test --headless --firefox
```

## Code Review Expectations

- Maintainers will review for correctness, code quality, and adherence to project standards
- Be open to feedback and willing to make changes
- Reviews may take time - be patient
- Not all PRs will be accepted, even if well-written

## Documentation

- Public APIs must have doc comments
- Complex algorithms or non-obvious code should have inline comments explaining the "why"
- Update relevant documentation in the repository when adding features

## Community Guidelines

- Be respectful and professional in all interactions
- Follow the [Matrix community guidelines](https://matrix.to/#/#nexum:nxm.rs)
- Issues and PRs are for actionable items - use Discussions for open-ended questions

## Getting Help

- **Matrix**: Join the [Nexum Matrix space](https://matrix.to/#/#nexum:nxm.rs) for real-time discussion
- **Discussions**: Use [GitHub Discussions](https://github.com/nxm-rs/nexum/discussions) for design questions and feature proposals
- **Issues**: Report bugs or propose accepted features via [GitHub Issues](https://github.com/nxm-rs/nexum/issues)

---

## Development Setup Reference

### Required Tools

1. [Rust toolchain](https://www.rust-lang.org/tools/install) (MSRV: 1.88)
2. [`wasm-pack`](https://github.com/rustwasm/wasm-pack) - For building WASM modules
3. [`trunk`](https://trunkrs.dev/) - For building the browser UI
4. [`just`](https://just.systems/) - Task runner (optional but recommended)
5. [`web-ext`](https://github.com/mozilla/web-ext) - For running the extension (optional)

### Building

#### Browser Extension

```sh
# Build the extension (creates crates/nexum/extension/dist/)
just build-ext

# Run the extension with web-ext (auto-reload on changes)
just run-ext

# Package the extension for distribution
just pack-ext
```

#### Terminal Interface (TUI)

```sh
# Build the TUI
cargo build -p tui

# Run the TUI
cargo run -p tui

# Or run directly after building
./target/debug/tui
```

#### RPC Server

```sh
# Build the RPC server
cargo build -p nexum-rpc

# Run with custom configuration
cargo run -p nexum-rpc -- --help
```

#### Keycard CLI

```sh
# Build the keycard CLI
cargo build -p keycard-cli

# Run keycard commands
cargo run -p keycard-cli -- --help
```

### Common Development Commands

```sh
# Run clippy on all code
just clippy
# or: cargo clippy --all-targets --all-features --workspace -- -Dwarnings

# Run all tests
just test
# or: cargo test --all-targets --all-features --workspace

# Build everything
just build
# or: cargo build --all-targets --all-features --workspace

# Check WASM-specific code (extension components)
cargo check --target wasm32-unknown-unknown -p worker -p injected -p injector -p browser-ui
```

### Project Structure

See [CLAUDE.md](./CLAUDE.md) and [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) for detailed architecture documentation and guidance on navigating the codebase.
