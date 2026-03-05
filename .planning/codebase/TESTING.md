# Testing Patterns

**Analysis Date:** 2026-03-05

## Current State

**No tests exist in this codebase.** There are:
- Zero `#[test]` annotations in any Rust source file
- Zero `#[cfg(test)]` modules
- No test files (`*_test.rs`, `test_*.rs`)
- No integration test directory (`tests/`)
- No test configuration or CI pipeline

## Test Framework

**Runner:**
- Rust's built-in test framework (via `cargo test`)
- No additional test dependencies in `Cargo.toml`

**Run Commands:**
```bash
cargo test                # Run all tests (currently zero)
cargo test -- --nocapture # Run with stdout visible
```

## Recommended Test Structure

Based on the codebase architecture, tests should follow these patterns.

### Unit Test Location

Use inline `#[cfg(test)]` modules in each source file (Rust convention):

```
src/
├── store.rs           # Add #[cfg(test)] mod tests at bottom
├── models/
│   └── contact.rs     # Add #[cfg(test)] mod tests at bottom
└── commands/
    ├── add.rs         # Add #[cfg(test)] mod tests at bottom
    └── ...
```

### Integration Test Location

Create a `tests/` directory at the project root:

```
tests/
├── cli_tests.rs       # End-to-end CLI tests via Command
└── fixtures/
    └── test-contact.md # Sample contact files for parsing
```

## Priority Test Areas

### 1. Frontmatter Parsing (`src/store.rs`)

The `parse_frontmatter()` function is the most critical testable unit. It handles YAML extraction from markdown and is private, so test via `parse_contact_file()`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_valid_contact() {
        let content = "---\nid: \"abc\"\nname: \"Test User\"\n---\n\n## Notes\n";
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{}", content).unwrap();

        let cf = parse_contact_file(tmp.path()).unwrap();
        assert_eq!(cf.contact.name, "Test User");
        assert_eq!(cf.contact.id, "abc");
    }

    #[test]
    fn test_parse_missing_frontmatter() {
        let content = "No frontmatter here";
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{}", content).unwrap();

        assert!(parse_contact_file(tmp.path()).is_err());
    }
}
```

### 2. Contact Slug Generation (`src/models/contact.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_basic() {
        let c = Contact { name: "Jane Smith".to_string(), ..Default::default() };
        assert_eq!(c.slug(), "jane-smith");
    }

    #[test]
    fn test_slug_special_chars() {
        let c = Contact { name: "José María".to_string(), ..Default::default() };
        assert_eq!(c.slug(), "jos-mara");  // Note: strips non-ASCII
    }
}
```

**Note:** `Contact` does not currently derive `Default`, which would be needed for test ergonomics. Consider adding `#[derive(Default)]` or a test helper constructor.

### 3. Serialization Roundtrip (`src/store.rs`)

```rust
#[test]
fn test_serialize_roundtrip() {
    // Create a ContactFile, serialize, parse back, assert equality
}
```

### 4. Contact Matching Logic

The name-matching pattern (lowercase substring) is duplicated across `src/commands/show.rs`, `src/commands/log.rs`, and `src/commands/search.rs`. Extract and test independently.

## Mocking Considerations

**Filesystem:**
- All commands depend on `store::find_crm_root()` and `store::load_all_contacts()`
- Tests should use `tempdir` with known contact files rather than mocking
- Set `ACRM_ROOT` env var to point to a test directory

**Time:**
- `src/commands/log.rs` and `src/commands/due.rs` use `chrono::Local::now()`
- For deterministic tests, consider accepting a date parameter or using a clock abstraction

**No external services to mock.** This is a local-only CLI tool.

## Test Dependencies to Add

When adding tests, add these to `Cargo.toml` under `[dev-dependencies]`:

```toml
[dev-dependencies]
tempfile = "3"        # Temporary files/directories for filesystem tests
assert_cmd = "2"      # CLI integration testing
predicates = "3"      # Assertion helpers for CLI output
```

## Coverage

**Requirements:** None enforced.

**View Coverage:**
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Shell Script Testing

The scripts in `scripts/` have no test coverage. They could be tested via:
- `bats` (Bash Automated Testing System)
- Or simply validated by the Rust CLI which duplicates their functionality

Since the Rust CLI (`acrm`) provides the same features as the shell scripts, prioritize testing the Rust code.

---

*Testing analysis: 2026-03-05*
