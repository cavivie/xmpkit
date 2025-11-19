# Contributing to XMPKit

Thank you for your interest in contributing to XMPKit! This document provides guidelines for contributing to the project.

## Code Style

- Follow Rust standard formatting: run `cargo fmt` before committing
- Follow Rust linting guidelines: run `cargo clippy` and fix any warnings
- Use meaningful variable and function names
- Add comments for complex logic or non-obvious behavior

## Testing

- Add tests for all new features
- Ensure all existing tests pass: `cargo test`
- Test edge cases and error conditions
- Update tests when modifying existing functionality

## Documentation

- Update documentation for all public API changes
- Add doc comments for new public functions, structs, and enums
- Include usage examples in doc comments when helpful
- Update README.md if adding new features or changing behavior

## Pull Requests

### Before Submitting

1. **Fork the repository** and create a branch from `main`
2. **Make your changes** following the code style guidelines
3. **Add tests** for your changes
4. **Run tests** to ensure everything passes: `cargo test`
5. **Run clippy** to check for linting issues: `cargo clippy`
6. **Format code**: `cargo fmt`

### PR Guidelines

- **Keep PRs focused**: One feature or fix per PR
- **Write clear commit messages**: Use conventional commit format when possible
- **Include tests**: All new features should have tests
- **Update documentation**: Update relevant documentation for API changes
- **Ensure CI passes**: All CI checks must pass before requesting review

### PR Description

Include in your PR description:
- What changes you made and why
- How to test the changes
- Any breaking changes (if applicable)
- Related issues (if any)

## Major Changes

For major changes or new features:
1. **Open an issue first** to discuss the proposed changes
2. Get feedback before implementing
3. This helps avoid wasted effort and ensures alignment with project goals

## Development Setup

1. Clone the repository
2. Install Rust (latest stable version recommended)
3. Run `cargo test` to verify everything works
4. Run `cargo build` to build the project

## Questions?

If you have questions about contributing, feel free to:
- Open an issue with the `question` label
- Check existing issues and discussions

Thank you for contributing to XMPKit! 🎉

