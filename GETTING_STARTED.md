# Getting Started with CodeGraph Development

**You're all set up! Here's what to do next.**

---

## ✅ What's Done

The project structure is complete and compiles successfully:
- ✅ Rust project initialized
- ✅ All dependencies configured
- ✅ CLI framework ready
- ✅ Module structure in place
- ✅ Documentation created

---

## 🎯 Week 1 Goals (This Week)

Your goal for this week: **Get basic indexing working**

### Day 1-2: SQLite Schema (IN PROGRESS)

Create the database schema:

```bash
# Edit this file:
vim src/index/schema.rs
```

Implement:
- [ ] Symbols table
- [ ] Relationships table
- [ ] Migrations
- [ ] Basic CRUD operations

**Test it:**
```bash
cargo test index::schema
```

### Day 3-4: File Watcher

Implement file watching:

```bash
# Edit this file:
vim src/indexer/watcher.rs
```

Implement:
- [ ] Watch directory for changes
- [ ] Detect file modifications
- [ ] Trigger re-indexing

**Test it:**
```bash
cargo run -- index --project . --watch
```

### Day 5-6: Basic Parser

Integrate Intent parser:

```bash
# Edit this file:
vim src/indexer/parser.rs
```

Implement:
- [ ] Parse Intent files
- [ ] Extract function names
- [ ] Store in database

**Test it:**
```bash
cargo run -- index --project ../intent-transpiler
cargo run -- query callees "main"
```

### Day 7: Week 1 Demo

**Goal:** Index 1 Intent file and query 1 symbol

```bash
# This should work by end of week:
echo "fn hello() { println(\"Hello\") }" > test.intent
cargo run -- index --project .
cargo run -- query definitions "hello"
# Output: test.intent:1:4 - fn hello()
```

---

## 🚀 Commands to Get Started

### Build & Test

```bash
# Build (should already work)
cargo build

# Run tests (none yet, but should pass)
cargo test

# Check code
cargo clippy

# Format code
cargo fmt
```

### Try the CLI

```bash
# See help
cargo run -- --help

# Try commands (stub implementations)
cargo run -- languages
cargo run -- stats
```

---

## 📝 Development Workflow

### 1. Create a Branch

```bash
git checkout -b week1/sqlite-schema
```

### 2. Write Code

Focus on one file at a time:
- Start with `src/index/schema.rs`
- Then `src/index/db.rs`
- Then `src/indexer/watcher.rs`

### 3. Test

```bash
cargo test
```

### 4. Commit

```bash
git add .
git commit -m "feat: implement SQLite schema for symbols"
```

---

## 🎓 Learning Resources

### Rust + SQLite
- [rusqlite docs](https://docs.rs/rusqlite/)
- [SQLite tutorial](https://www.sqlitetutorial.net/)

### File Watching
- [notify crate](https://docs.rs/notify/)

### MCP Protocol
- [MCP docs](https://modelcontextprotocol.io/)
- [MCP examples](https://github.com/modelcontextprotocol/servers)

---

## 💡 Tips

### Start Small
Don't try to implement everything at once. Focus on:
1. Store 1 symbol in SQLite ✅
2. Query that symbol ✅
3. Then expand

### Use Examples
Look at the Intent transpiler for parsing examples:
```bash
cd ../intent-transpiler
rg "fn parse" src/parser.rs
```

### Ask Questions
- Open GitHub Discussion
- Comment on roadmap issues

---

## 🐛 If You Get Stuck

### Build Errors

```bash
# Clean build
cargo clean
cargo build

# Update dependencies
cargo update
```

### Test Failures

```bash
# Run specific test
cargo test test_name -- --nocapture

# Show output
cargo test -- --show-output
```

### IDE Setup

**VSCode:**
- Install "rust-analyzer" extension
- Open workspace: `code .`
- IDE will show errors inline

**vim/neovim:**
- Use rust-analyzer LSP
- `:CocInstall coc-rust-analyzer`

---

## 📊 Progress Tracking

Update ROADMAP.md as you complete tasks:

```markdown
### Week 1-2: Core Infrastructure
- [x] Project setup
- [x] CLI interface
- [x] Module structure
- [ ] SQLite schema  ← Mark done when complete
- [ ] File watcher
```

---

## 🎉 When Week 1 is Done

You'll have:
- Working SQLite index
- File watcher detecting changes
- Basic Intent parser
- Can index 1 file and query 1 symbol

**That's 80% of the hard work!** Week 2-8 is expanding this foundation.

---

## 📞 Need Help?

- **Code questions:** Open GitHub Discussion
- **Bug reports:** Open GitHub Issue
- **Urgent:** Message on Discord

---

**Ready to code? Start with `src/index/schema.rs`!** 🚀

```bash
vim src/index/schema.rs
```

Good luck! You've got this. 💪
