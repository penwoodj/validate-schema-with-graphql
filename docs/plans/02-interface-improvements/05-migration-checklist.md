# Migration Checklist

**For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide a step-by-step migration checklist from the existing `gqlsdl-*` crates to the new `graphql-ish-schema-validator` ecosystem with file-by-file changes, import path updates, test updates, and verification steps.

**Architecture:** Phased migration (restructure → API → CLI → tests → docs) with verification after each phase.

**Tech Stack:** Rust, Cargo, git, documentation tools

---

## Context and Rationale

**Migration scope:**
- Rename all crates from `gqlsdl-*` to `graphql-ish-schema-validator-*`
- Update all import paths throughout the codebase
- Update public API to use new crate names
- Update tests to use new imports
- Update documentation and examples
- Verify functionality at each step

**Migration strategy:**
1. **Phase 1**: Crate renaming and restructuring
2. **Phase 2**: Public API updates
3. **Phase 3**: CLI updates
4. **Phase 4**: Test updates
5. **Phase 5**: Documentation updates
6. **Phase 6**: Final verification

**Prerequisites:**
- All new crate structures created (from 00-rename-and-restructure.md)
- Public API designed (from 01-public-api.md)
- CLI implemented (from 03-cli-improvements.md)
- Tests written (from 04-testing-and-verification.md)

---

## Phase 1: Crate Restructuring

### Step 1: Create New Crate Directories

```bash
# Create new crate directories
mkdir -p crates/graphql-ish-schema-validator-ir/src
mkdir -p crates/graphql-ish-schema-validator-parser/src
mkdir -p crates/graphql-ish-schema-validator-validator/src
mkdir -p crates/graphql-ish-schema-validator-registry/src
mkdir -p crates/graphql-ish-schema-validator-cli/src
mkdir -p crates/graphql-ish-schema-validator/src
```

- [ ] Created new crate directories

### Step 2: Update Workspace Cargo.toml

Update `Cargo.toml` to reference new crate names:

```toml
[workspace]
resolver = "2"
members = [
    "crates/graphql-ish-schema-validator",
    "crates/graphql-ish-schema-validator-ir",
    "crates/graphql-ish-schema-validator-parser",
    "crates/graphql-ish-schema-validator-validator",
    "crates/graphql-ish-schema-validator-registry",
    "crates/graphql-ish-schema-validator-cli",
]
```

- [ ] Updated workspace manifest

### Step 3: Copy IR Crate

```bash
# Copy existing IR code
cp crates/gqlsdl-ir/src/lib.rs crates/graphql-ish-schema-validator-ir/src/lib.rs
cp crates/gqlsdl-ir/src/*.rs crates/graphql-ish-schema-validator-ir/src/ 2>/dev/null || true

# Update package name in Cargo.toml
sed 's/name = "gqlsdl-ir"/name = "graphql-ish-schema-validator-ir"/g' \
    crates/gqlsdl-ir/Cargo.toml > crates/graphql-ish-schema-validator-ir/Cargo.toml

# Update dependencies
sed -i 's/gqlsdl-ir/graphql-ish-schema-validator-ir/g' \
    crates/graphql-ish-schema-validator-ir/Cargo.toml
```

- [ ] Copied IR crate code
- [ ] Updated IR package name
- [ ] Updated IR dependencies

### Step 4: Copy Parser Crate

```bash
# Copy existing parser code
cp crates/sdl-parser/src/lib.rs crates/graphql-ish-schema-validator-parser/src/lib.rs
cp crates/sdl-parser/src/*.rs crates/graphql-ish-schema-validator-parser/src/ 2>/dev/null || true

# Update package name in Cargo.toml
sed 's/name = "sdl-parser"/name = "graphql-ish-schema-validator-parser"/g' \
    crates/sdl-parser/Cargo.toml > crates/graphql-ish-schema-validator-parser/Cargo.toml

# Update dependencies
sed -i 's/gqlsdl-ir/graphql-ish-schema-validator-ir/g' \
    crates/graphql-ish-schema-validator-parser/Cargo.toml
```

- [ ] Copied parser crate code
- [ ] Updated parser package name
- [ ] Updated parser dependencies

### Step 5: Copy Validator Crate

```bash
# Copy existing validator code
cp crates/validator/src/lib.rs crates/graphql-ish-schema-validator-validator/src/lib.rs
cp crates/validator/src/*.rs crates/graphql-ish-schema-validator-validator/src/ 2>/dev/null || true

# Update package name in Cargo.toml
sed 's/name = "validator"/name = "graphql-ish-schema-validator-validator"/g' \
    crates/validator/Cargo.toml > crates/graphql-ish-schema-validator-validator/Cargo.toml

# Update dependencies
sed -i 's/gqlsdl-ir/graphql-ish-schema-validator-ir/g' \
    crates/graphql-ish-schema-validator-validator/Cargo.toml
sed -i 's/gqlsdl-parser/graphql-ish-schema-validator-parser/g' \
    crates/graphql-ish-schema-validator-validator/Cargo.toml
```

- [ ] Copied validator crate code
- [ ] Updated validator package name
- [ ] Updated validator dependencies

### Step 6: Verify Phase 1

```bash
# Try to build all new crates
cargo check --workspace

# Verify no references to old crate names remain
grep -r "gqlsdl-ir" crates/graphql-ish-schema-validator-*/src/ 2>/dev/null || echo "No old IR references"
grep -r "gqlsdl-parser" crates/graphql-ish-schema-validator-*/src/ 2>/dev/null || echo "No old parser references"
```

- [ ] All new crates build successfully
- [ ] No old crate name references remain

### Step 7: Commit Phase 1

```bash
git add Cargo.toml crates/
git commit -m "migrate(phase1): restructure crates with new naming"
```

- [ ] Phase 1 committed

---

## Phase 2: Import Path Updates

### Step 1: Update IR Module Imports

Search and replace IR imports:

```bash
# In parser crate
sed -i 's/use gqlsdl_ir::/use graphql_ish_schema_validator_ir::/g' \
    crates/graphql-ish-schema-validator-parser/src/*.rs

# In validator crate
sed -i 's/use gqlsdl_ir::/use graphql_ish_schema_validator_ir::/g' \
    crates/graphql-ish-schema-validator-validator/src/*.rs
```

- [ ] Updated IR imports in parser
- [ ] Updated IR imports in validator

### Step 2: Update Parser Module Imports

Search and replace parser imports:

```bash
# In validator crate
sed -i 's/use gqlsdl_parser::/use graphql_ish_schema_validator_parser::/g' \
    crates/graphql-ish-schema-validator-validator/src/*.rs
```

- [ ] Updated parser imports in validator

### Step 3: Update External Dependencies

Check and update any external dependencies:

```bash
# Check for any remaining old crate references
grep -r "gqlsdl_" crates/ --include="*.rs" --include="*.toml" 2>/dev/null || echo "All imports updated"
```

- [ ] All external dependencies updated

### Step 4: Verify Phase 2

```bash
# Build all crates
cargo build --workspace

# Run basic smoke test
cargo test --workspace --lib
```

- [ ] All crates compile
- [ ] Basic tests pass

### Step 5: Commit Phase 2

```bash
git add crates/
git commit -m "migrate(phase2): update all import paths to new crate names"
```

- [ ] Phase 2 committed

---

## Phase 3: Public API Updates

### Step 1: Create Top-Level Crate

```bash
# Copy the public API stub from the plan
cp docs/plans/02-interface-improvements/01-public-api.md /tmp/public_api_plan.md

# Extract the lib.rs content and create the file
# (This would be done manually or via script)
```

- [ ] Top-level crate created

### Step 2: Update Top-Level Cargo.toml

Ensure `crates/graphql-ish-schema-validator/Cargo.toml` has correct dependencies:

```toml
[package]
name = "graphql-ish-schema-validator"
version.workspace = true
edition.workspace = true

[dependencies]
graphql-ish-schema-validator-ir = { path = "../graphql-ish-schema-validator-ir" }
graphql-ish-schema-validator-parser = { path = "../graphql-ish-schema-validator-parser" }
graphql-ish-schema-validator-validator = { path = "../graphql-ish-schema-validator-validator" }
thiserror = { workspace = true }
tracing = { workspace = true }

[features]
default = ["yaml", "cli"]
yaml = ["graphql-ish-schema-validator-validator"]
json-schema-export = []
http-registry = []
cli = ["graphql-ish-schema-validator-cli"]
```

- [ ] Top-level Cargo.toml updated

### Step 3: Update Examples

Update example code to use new API:

```bash
# Find and update examples
find examples/ -name "*.rs" -exec sed -i 's/use gqlsdl::/use graphql_ish_schema_validator::/g' {} \;
find examples/ -name "*.rs" -exec sed -i 's/use gqlsdl_validator::/use graphql_ish_schema_validator::/g' {} \;
```

- [ ] All examples updated

### Step 4: Verify Phase 3

```bash
# Build top-level crate
cargo build -p graphql-ish-schema-validator

# Run example
cargo run --example basic_validation
```

- [ ] Top-level crate builds
- [ ] Examples run successfully

### Step 5: Commit Phase 3

```bash
git add crates/graphql-ish-schema-validator/ examples/
git commit -m "migrate(phase3): create top-level public API crate"
```

- [ ] Phase 3 committed

---

## Phase 4: CLI Updates

### Step 1: Create CLI Crate Structure

```bash
# Create CLI crate from stub
cp docs/plans/02-interface-improvements/03-cli-improvements.md /tmp/cli_plan.md

# Extract main.rs content (manual or scripted)
```

- [ ] CLI crate structure created

### Step 2: Update CLI Cargo.toml

Ensure `crates/graphql-ish-schema-validator-cli/Cargo.toml` is correct:

```toml
[package]
name = "graphql-ish-schema-validator-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "graphql-ish-schema-validator"
path = "src/main.rs"

[[bin]]
name = "gqlsdl"
path = "src/main.rs"

[dependencies]
graphql-ish-schema-validator = { path = "../graphql-ish-schema-validator", features = ["yaml"] }
anyhow = "1.0"
clap = { version = "4.4", features = ["derive"] }
# ... other dependencies
```

- [ ] CLI Cargo.toml updated

### Step 3: Update CLI Scripts

Update any shell scripts or build scripts:

```bash
# Find and update CLI references in scripts
find scripts/ -type f -exec sed -i 's/gqlsdl-validate/graphql-ish-schema-validator/g' {} \;
find scripts/ -type f -exec sed -i 's/gqlsdl-jtd/graphql-ish-schema-validator/g' {} \;
```

- [ ] All CLI scripts updated

### Step 4: Verify Phase 4

```bash
# Build CLI
cargo build -p graphql-ish-schema-validator-cli --release

# Test CLI help
cargo run -p graphql-ish-schema-validator-cli -- --help

# Test short alias
cargo run -p graphql-ish-schema-validator-cli -- --help 2>&1 | grep graphql-ish-schema-validator
```

- [ ] CLI builds successfully
- [ ] Help text displays correctly
- [ ] Binary names work (both full and short)

### Step 5: Commit Phase 4

```bash
git add crates/graphql-ish-schema-validator-cli/ scripts/
git commit -m "migrate(phase4): create CLI with new binary names"
```

- [ ] Phase 4 committed

---

## Phase 5: Test Updates

### Step 1: Update Integration Test Paths

```bash
# Find and update test imports
find tests/ -name "*.rs" -exec sed -i 's/use gqlsdl::/use graphql_ish_schema_validator::/g' {} \;
find tests/ -name "*.rs" -exec sed -i 's/use gqlsdl_validator::/use graphql_ish_schema_validator_validator::/g' {} \;
find crates/*/tests/ -name "*.rs" -exec sed -i 's/use gqlsdl::/use graphql_ish_schema_validator::/g' {} \;
```

- [ ] All test imports updated

### Step 2: Update Benchmark Imports

```bash
# Update benchmark imports
find benches/ -name "*.rs" -exec sed -i 's/use gqlsdl::/use graphql_ish_schema_validator::/g' {} \;
```

- [ ] All benchmark imports updated

### Step 3: Fix Test File Paths

Check for any hardcoded paths in tests:

```bash
# Look for references to old crate names in tests
grep -r "gqlsdl" tests/ crates/*/tests/ 2>/dev/null || echo "No old references in tests"
```

- [ ] All test file paths updated

### Step 4: Verify Phase 5

```bash
# Run all tests
cargo test --workspace

# Run benchmarks
cargo bench -p graphql-ish-schema-validator-validator

# Run integration tests
cargo test --test cli_tests
```

- [ ] All tests pass
- [ ] Benchmarks run successfully
- [ ] Integration tests pass

### Step 5: Commit Phase 5

```bash
git add tests/ benches/
git commit -m "migrate(phase5): update all test imports and paths"
```

- [ ] Phase 5 committed

---

## Phase 6: Documentation Updates

### Step 1: Update README.md

Update main README to reference new names:

```bash
# Replace crate names in README
sed -i 's/gqlsdl-validator/graphql-ish-schema-validator/g' README.md
sed -i 's/gqlsdl-jtd/graphql-ish-schema-validator/g' README.md
sed -i 's/gqlsdl-validate/graphql-ish-schema-validator validate/g' README.md
```

- [ ] README.md updated

### Step 2: Update Code Examples

Update all code examples in documentation:

```bash
# Update examples in docs/
find docs/ -name "*.md" -exec sed -i 's/use gqlsdl::/use graphql_ish_schema_validator::/g' {} \;
find docs/ -name "*.md" -exec sed -i 's/validate_yaml(/validate_yaml_from_schema(/g' {} \;
find docs/ -name "*.md" -exec sed -i 's/validate_json(/validate_json_from_schema(/g' {} \;
```

- [ ] All code examples in docs updated

### Step 3: Update Cross-References

Update cross-references between plan documents:

```bash
# Update plan cross-refs
find docs/plans/ -name "*.md" -exec sed -i 's/gqlsdl-validator/graphql-ish-schema-validator/g' {} \;
find docs/plans/ -name "*.md" -exec sed -i 's/gqlsdl-ir/graphql-ish-schema-validator-ir/g' {} \;
```

- [ ] All cross-references updated

### Step 4: Update API Documentation

Regenerate API documentation:

```bash
# Build documentation
cargo doc --workspace --no-deps

# Check for warnings
# (manual review needed)
```

- [ ] API documentation builds
- [ ] No major warnings

### Step 5: Verify Phase 6

```bash
# Build all documentation
cargo doc --workspace

# Check README examples compile
# (manual verification)
```

- [ ] All documentation updated
- [ ] README examples compile

### Step 6: Commit Phase 6

```bash
git add README.md docs/
git commit -m "migrate(phase6): update all documentation and examples"
```

- [ ] Phase 6 committed

---

## Phase 7: Final Verification

### Step 1: Full Workspace Build

```bash
# Clean build
cargo clean

# Build everything
cargo build --workspace --release
```

- [ ] Full workspace builds successfully

### Step 2: Full Test Suite

```bash
# Run all tests
cargo test --workspace

# Run with coverage (if configured)
./scripts/coverage.sh || echo "Coverage script not configured"
```

- [ ] All tests pass
- [ ] Coverage meets targets

### Step 3: CLI Smoke Tests

```bash
# Test all subcommands
cargo run -p graphql-ish-schema-validator-cli -- --help
cargo run -p graphql-ish-schema-validator-cli -- validate --help
cargo run -p graphql-ish-schema-validator-cli -- compile --help
cargo run -p graphql-ish-schema-validator-cli -- check-schema --help
cargo run -p graphql-ish-schema-validator-cli -- export-jtd --help

# Test with real files
# (manual verification with test fixtures)
```

- [ ] All CLI subcommands work
- [ ] Help text is correct
- [ ] Binary names work

### Step 4: Public API Tests

```bash
# Test API usage
cargo run --example basic_validation
cargo run --example advanced_options
cargo run --example error_handling
cargo run --example logging_example
cargo run --example output_formats
```

- [ ] All examples run successfully
- [ ] Public API works as expected

### Step 5: Verify No Old References

```bash
# Search for any remaining old crate names
grep -r "gqlsdl-" crates/ --include="*.rs" --include="*.toml" --include="*.md" 2>/dev/null || echo "No old crate name references"
grep -r "gqlsdl_diagnostics" crates/ --include="*.rs" --include="*.toml" 2>/dev/null || echo "No old diagnostic references"
grep -r "gqlsdl_validator" crates/ --include="*.rs" --include="*.toml" 2>/dev/null || echo "No old validator references"
```

- [ ] No old crate name references remain

### Step 6: Performance Verification

```bash
# Run benchmarks
cargo bench -p graphql-ish-schema-validator-validator

# Verify targets met
# SDL compile: <10ms
# Validation: <50ms for 10K nodes
```

- [ ] Performance targets met
- [ ] No regressions

### Step 7: Git History Review

```bash
# Review migration commits
git log --oneline --grep="migrate"

# Verify clean history
git status
```

- [ ] Migration commits are clean
- [ ] Working directory is clean

### Step 8: Final Commit

```bash
# Tag the migration
git tag -a v0.2.0-migration -m "Migrate to graphql-ish-schema-validator ecosystem"

# Push (when ready)
# git push origin main
# git push origin v0.2.0-migration
```

- [ ] Migration tagged
- [ ] Ready for review/release

---

## Rollback Plan

If issues are discovered during migration, rollback steps:

### Partial Rollback

```bash
# Reset to specific phase
git reset --hard <phase-commit>

# Or reset to before migration
git reset --hard <pre-migration-commit>
```

### Full Rollback

```bash
# Delete migration tag (if pushed)
git tag -d v0.2.0-migration

# Reset all changes
git reset --hard <pre-migration-commit>

# Verify old state works
cargo test --workspace
```

---

## Migration Summary Checklist

### Before Migration
- [ ] All new plan files reviewed and approved
- [ ] Feature branch created for migration
- [ ] Backup of current working state
- [ ] Tests passing on current codebase

### During Migration
- [ ] Phase 1: Crate restructuring completed
- [ ] Phase 2: Import paths updated
- [ ] Phase 3: Public API created
- [ ] Phase 4: CLI updated
- [ ] Phase 5: Tests updated
- [ ] Phase 6: Documentation updated
- [ ] Phase 7: Final verification passed

### After Migration
- [ ] All tests pass
- [ ] CI/CD green
- [ ] Documentation builds
- [ ] Examples work
- [ ] Performance targets met
- [ ] No regressions detected
- [ ] Migration tagged
- [ ] Release notes prepared

### Cleanup
- [ ] Old crate directories removed (if verified working)
- [ ] Old branches deleted
- [ ] Migration plan archived
- [ ] Team notified of completion

---

## Success Criteria

Migration is successful when:

1. **All crates renamed**: No `gqlsdl-*` names in codebase
2. **All imports updated**: Code compiles without errors
3. **All tests pass**: Full test suite passes
4. **CLI works**: All subcommands functional with new binary names
5. **Documentation updated**: README and docs reference new names
6. **Performance maintained**: No performance regressions
7. **CI/CD green**: All automated checks pass
8. **No old references**: Clean migration, no leftover references

---

## Known Issues and Workarounds

### Issue: Circular Dependencies
**Symptom**: Cargo complains about circular dependencies between new crates

**Solution**: Ensure dependency graph is acyclic:
- Top-level crate depends on sub-crates
- Sub-crates do NOT depend on top-level crate
- Parser and IR are independent
- Validator depends on IR and Parser
- CLI depends on Validator

### Issue: Test Failures After Import Updates
**Symptom**: Tests fail after updating imports

**Solution**: Check for:
- Updated all test imports
- Updated all fixture paths
- Updated all module declarations
- Updated all use statements

### Issue: CLI Binary Not Found
**Symptom**: `graphql-ish-schema-validator` command not found

**Solution**: Install with:
```bash
cargo install --path crates/graphql-ish-schema-validator-cli
# Or use binary directly:
cargo run -p graphql-ish-schema-validator-cli -- <args>
```

---

## Resources

- **Plan documents**: `docs/plans/02-interface-improvements/*.md`
- **Old crates**: `crates/gqlsdl-*/` (to be removed after verification)
- **New crates**: `crates/graphql-ish-schema-validator-*/`
- **Migration branch**: `feature/interface-improvements-migration`
- **Pre-migration baseline**: Tag `v0.1.0-pre-migration`

---

## Communication

**Team notification**: Send migration summary with:
- Changes made
- Breaking changes
- Migration instructions for downstream users
- Timeline for removal of old crate names

**Downstream notification**: Inform users of:
- New crate names
- Updated installation instructions
- Deprecation timeline for old names

---

## Support

During migration, use these resources for help:
- **Plan documents**: Detailed steps and rationale
- **Test suite**: Run tests to verify changes
- **Documentation**: Reference updated docs
- **Git history**: Check migration commits for context

---

## Next Steps After Migration

1. **Monitor**: Watch for issues reported by users
2. **Support**: Help users migrate their code
3. **Deprecate**: Announce deprecation of old names
4. **Remove**: Remove old crates after deprecation period (e.g., 3 months)
5. **Iterate**: Continue implementation based on other plans
