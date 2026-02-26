# Contributing to swapx

Thanks for your interest in contributing! This guide will help you get started.

## Development Setup

```sh
# Clone the repo
git clone https://github.com/BeepBoopBit/swapx.git
cd swapx

# Build
cargo build

# Run tests
cargo test

# Run all checks (format, lint, test)
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Making Changes

1. Fork the repo and create a feature branch from `main`.
2. Make your changes with clear, focused commits.
3. Ensure all checks pass:
   ```sh
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```
4. Open a pull request against `main`.

## Pull Request Process

- Keep PRs focused — one feature or fix per PR.
- Update documentation if your change affects user-facing behavior.
- Add tests for new functionality.
- Fill out the PR template checklist.

## Adding Rules and Tests

### Adding example rules

Add examples to `EXAMPLES.md` following the existing format. Each example should include:
- A description of the use case
- The YAML rule configuration
- A before/after demonstration

### Adding tests

- **Unit tests** go in the relevant `src/*.rs` file inside a `#[cfg(test)] mod tests` block.
- **Integration tests** go in the `tests/` directory.
- Test both the happy path and edge cases.

## Coding Standards

- Run `cargo fmt` before committing.
- No `clippy` warnings — treat them as errors.
- Follow existing code patterns and naming conventions.
- Keep dependencies minimal — don't add a crate for something the standard library handles.

## Reporting Bugs

Use the [bug report template](https://github.com/BeepBoopBit/swapx/issues/new?template=bug_report.md) on GitHub.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold it.
