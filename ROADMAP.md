# CodeGraph Development Roadmap

**8-Week MVP → Production → Ecosystem**

---

## Phase 1: MVP (Weeks 1-8)

### Week 1-2: Core Infrastructure ✅ IN PROGRESS
- [x] Project setup (Rust, Cargo)
- [x] CLI interface
- [x] Module structure
- [ ] SQLite schema
- [ ] File watcher (notify crate)
- [ ] Basic configuration

### Week 3-4: Intent Parser & Indexer
- [ ] Intent parser integration
- [ ] Symbol extraction (contexts, functions, types)
- [ ] Relationship extraction (calls, references)
- [ ] Basic index building

### Week 5-6: Query Engine
- [ ] Implement 4 core queries:
  - callers
  - callees
  - references
  - dependencies
- [ ] Query optimization
- [ ] Result formatting
- [ ] Performance tuning (<100ms)

### Week 7-8: MCP Integration
- [ ] MCP server (stdio transport)
- [ ] Tool handlers
- [ ] Integration with Claude Desktop
- [ ] Documentation
- [ ] Demo video
- [ ] MVP Release

**Deliverable:** Working CodeGraph for Intent codebases

---

## Phase 2: Production (Weeks 9-16)

### Week 9-10: Advanced Queries
- [ ] Impact analysis
- [ ] Path finding
- [ ] Effects analysis
- [ ] Stats endpoint

### Week 11-12: Rust Support
- [ ] Rust parser (syn crate)
- [ ] Rust symbol extraction
- [ ] Cross-language references (Intent ↔ Rust)

### Week 13-14: Semantic Search
- [ ] Full-text search (FTS5)
- [ ] Natural language queries
- [ ] Relevance scoring

### Week 15-16: Polish & Release
- [ ] Error handling
- [ ] Performance tuning
- [ ] Production testing
- [ ] v1.0 Release

**Deliverable:** Production-ready CodeGraph (Intent + Rust)

---

## Phase 3: Ecosystem (Weeks 17-24)

### Week 17-20: Language Expansion
- [ ] TypeScript support
- [ ] Python support
- [ ] Language plugin system

### Week 21-22: Developer Experience
- [ ] CLI improvements
- [ ] Better error messages
- [ ] Configuration UI
- [ ] VSCode extension

### Week 23-24: Community
- [ ] Open source launch
- [ ] Documentation site
- [ ] Example projects
- [ ] Community building

**Deliverable:** Mature ecosystem, 4+ languages

---

## Success Metrics

### Month 2 (MVP)
- [ ] <100ms query latency
- [ ] Works with Intent codebase
- [ ] Integrated with Claude Desktop
- [ ] 10 alpha users

### Month 4 (Production)
- [ ] Supports Intent + Rust + TypeScript
- [ ] All 7 query types working
- [ ] 50 active users
- [ ] 100+ GitHub stars

### Month 6 (Ecosystem)
- [ ] 4+ languages supported
- [ ] 200 active users
- [ ] 500+ GitHub stars
- [ ] VSCode extension published

---

## Current Status: Week 1

**Completed:**
- ✅ Project scaffolding
- ✅ Cargo.toml with dependencies
- ✅ CLI interface (main.rs)
- ✅ Module structure
- ✅ Documentation (README, specs)

**Next Up (This Week):**
- [ ] SQLite schema design
- [ ] Database connection setup
- [ ] File watcher implementation
- [ ] Configuration system

**This Week's Goal:**
Can index 1 Intent file and store 1 symbol in SQLite

---

**Last Updated:** 2025-01-19
**Status:** ⚡ Active Development
