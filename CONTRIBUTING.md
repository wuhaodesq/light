# Contributing to Light Language

Thank you for your interest in contributing to Light Language!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/light.git`
3. Add the upstream: `git remote add upstream https://github.com/light-lang/light.git`
4. Create a branch: `git checkout -b feature/your-feature-name`

## Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- For embedded targets: appropriate cross-compilation toolchains

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
```

## Code Style

- Use `cargo fmt` before committing
- Run `cargo clippy` and address all warnings
- Write unit tests for new functionality
- Keep functions small and focused

## Commit Messages

- Use clear, descriptive commit messages
- Start with a verb (Add, Fix, Update, Remove)
- Reference issues when applicable

## Pull Request Process

1. Update documentation if needed
2. Add tests for new functionality
3. Ensure all checks pass (fmt, clippy, tests)
4. Update CHANGELOG.md
5. Request review from maintainers

## Areas to Contribute

- Lexer/Parser improvements
- More language features
- Better error messages
- Embedded target support
- VS Code extension enhancements
- Documentation

## Questions?

Open an issue for discussion before starting significant work.
