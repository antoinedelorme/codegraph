# Contributing to CodeGraph

Thank you for your interest in contributing to CodeGraph! This document provides guidelines and instructions for contributing.

---

## Getting Started

### Prerequisites

- Rust 1.70+ (stable)
- Git
- Basic understanding of:
  - Semantic code analysis
  - Graph databases
  - MCP protocol (helpful but not required)

### Setup Development Environment

```bash
# Clone the repository
git clone https://github.com/intent-lang/codegraph
cd codegraph

# Build the project
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- --help
```

---

## Development Workflow

### 1. Find an Issue

- Browse [GitHub Issues](https://github.com/intent-lang/codegraph/issues)
- Look for issues tagged `good first issue`
- Comment on the issue to let others know you're working on it

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

### 3. Make Your Changes

- Write clear, documented code
- Follow Rust conventions (use `cargo fmt` and `cargo clippy`)
- Add tests for new functionality
- Update documentation as needed

### 4. Test Your Changes

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Check formatting
cargo fmt --check

# Run linter
cargo clippy
```

### 5. Commit Your Changes

```bash
git add .
git commit -m "Clear description of changes"
```

**Commit Message Format:**
```
<type>: <description>

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

**Examples:**
```
feat: Add TypeScript parser support
fix: Correct query cache invalidation
docs: Update README with examples
test: Add tests for impact analysis
```

### 6. Push and Create Pull Request

```bash
git push origin your-branch-name
```

Then create a PR on GitHub with:
- Clear title and description
- Link to related issue
- Description of changes
- Testing performed

---

## Code Style

### Rust Conventions

- Use `cargo fmt` for formatting (enforced in CI)
- Use `cargo clippy` for linting (no warnings allowed)
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

### Documentation

- Add doc comments for public APIs:
  ```rust
  /// Queries the index for symbols matching the criteria.
  ///
  /// # Arguments
  /// * `query_type` - Type of query (callers, callees, etc.)
  /// * `target` - Target symbol name
  ///
  /// # Returns
  /// List of matching symbols with metadata
  pub fn query(query_type: QueryType, target: &str) -> Result<Vec<Symbol>> {
      // ...
  }
  ```

- Keep comments concise and up-to-date
- Use examples in documentation

### Testing

- Unit tests in same file as code:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_query_callers() {
          // ...
      }
  }
  ```

- Integration tests in `tests/` directory
- Benchmarks in `benches/` directory

---

## Project Structure

```
codegraph/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── cli/              # Command implementations
│   │   ├── serve.rs      # MCP server
│   │   ├── index.rs      # Indexing commands
│   │   ├── query.rs      # Query commands
│   │   └── ...
│   ├── index/            # Index storage
│   │   ├── schema.rs     # SQLite schema
│   │   └── db.rs         # Database operations
│   ├── indexer/          # Code parsing & indexing
│   │   ├── watcher.rs    # File watcher
│   │   └── parser.rs     # Language parsers
│   ├── query/            # Query engine
│   │   ├── engine.rs     # Query execution
│   │   └── cache.rs      # Query cache
│   └── mcp/              # MCP protocol
│       ├── server.rs     # MCP server
│       └── tools.rs      # Tool handlers
├── tests/                # Integration tests
├── benches/              # Performance benchmarks
└── docs/                 # Documentation
```

---

## Areas for Contribution

### Good First Issues

- **Add language support:**
  - Implement parser for Go, Java, C++
  - Add language-specific symbol extraction

- **Improve error messages:**
  - Add more context to errors
  - Suggest fixes for common problems

- **Documentation:**
  - Write tutorials
  - Add code examples
  - Improve API docs

- **Testing:**
  - Add unit tests
  - Write integration tests
  - Create benchmarks

### Advanced Contributions

- **Performance optimization:**
  - Query optimization
  - Index compression
  - Memory usage reduction

- **New features:**
  - Advanced query types
  - Better semantic search
  - Cross-repository indexing

- **Tooling:**
  - VSCode extension
  - LSP integration
  - CI/CD improvements

---

## Release Process

### Versioning

We follow [Semantic Versioning](https://semver.org/):
- **MAJOR:** Breaking changes
- **MINOR:** New features (backward compatible)
- **PATCH:** Bug fixes

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create git tag
5. Push to GitHub
6. Create GitHub release
7. Publish to crates.io (when ready)

---

## Communication

- **GitHub Issues:** Bug reports and feature requests
- **GitHub Discussions:** Questions and general discussion
- **Discord:** [Intent Community](https://discord.gg/intent-lang)

---

## Code of Conduct

Be respectful and constructive. We're all here to build something useful together.

- Be welcoming to newcomers
- Focus on what's best for the project
- Show empathy towards other contributors
- Accept constructive criticism gracefully

---

## Questions?

If you have questions, feel free to:
- Open a GitHub Discussion
- Ask in Discord
- Comment on relevant issues

Thank you for contributing to CodeGraph! 🎉
