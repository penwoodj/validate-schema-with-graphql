# kubeconform Behavioral Spec + miette API + Competitive Landscape

> Research by OpenCode librarian, 2026-04-15

## kubeconform Behavioral Specification

### `-strict` Flag

Source: [kubeconform/registry/registry.go](https://github.com/yannh/kubeconform/blob/e60892483e5b7e5dffa95fc3f121646a96ca270f/pkg/registry/registry.go)

- Adds `-strict` suffix to schema filenames in path templates
- Uses `yaml.UnmarshalStrict` instead of regular `yaml.Unmarshal`
- **Rejects**: additional properties, duplicate keys, unknown fields

### Schema Location System

- Default template: `https://raw.githubusercontent.com/yannh/kubernetes-json-schema/master/{{ .NormalizedKubernetesVersion }}-standalone{{ .StrictSuffix }}/{{ .ResourceKind }}{{ .KindSuffix }}.json`
- Template variables: `NormalizedKubernetesVersion`, `StrictSuffix`, `ResourceKind`, `ResourceAPIVersion`, `Group`, `KindSuffix`
- Multiple `-schema-location` flags searched in order
- If path doesn't end in `.json` → treated as Go template
- If ends in `.json` → specific schema file

### Caching

- Two-tier: in-memory + on-disk (`-cache` folder)
- `cache.Cache` interface with `Get(key)` / `Set(key, schema)`
- HTTP-downloaded schemas cached to avoid repeated fetches

### HTTP Security

- `-insecure-skip-tls-verify` to disable SSL (opt-in)
- Respects `HTTPS_PROXY` env var
- Default: validates SSL certificates

### Performance

- 5.3x faster than kubeval (6.7s vs 35.3s on 50,714 resources)
- Multi-core parallelization via goroutines, default 4 workers (`-n` flag)

## miette API Summary

### Status
- **Version**: 7.6.0 (2025-04-27)
- **Maintenance**: Active (65 releases, 4.17M downloads/month)
- **MSRV**: Rust 1.70.0+
- **License**: Apache-2.0

### Key Types
- `Diagnostic` trait — Main diagnostic protocol
- `Severity` — Error, Warning, Advice, etc.
- `SourceCode` — Generic source code handling
- `SourceSpan` — Location in source (byte offsets)

### thiserror Integration
```rust
#[derive(Error, Diagnostic, Debug)]
#[diagnostic(code(my_app::my_error), url(docsrs))]
#[error("kaboom")]
struct MyErr;
```

### Features
- Source code snippets with highlighting
- Multiple related errors per diagnostic
- Help text suggestions
- JSON output for tool integration
- `miette!` macro for inline diagnostics

## Competitive Landscape

| Tool | Language | Purpose | Downloads | Notes |
|---|---|---|---|---|
| **jsonschema** | Rust | JSON Schema validator | 59M total | Draft 4/6/7/2019-09/2020-12, 75-645x faster than valico |
| **serde_valid** | Rust | Serde-based validation | 2.13M/month | Derive macro, JSON Schema semantics |
| **valico** | Rust | JSON Schema + coercer | Lower | Inactive (last commit 2024-03) |
| **yaml-schema** | Rust | YAML Schema validator | 5.9K total | CLI + library, JSON Schema-based |
| **taplo** | Rust | TOML toolkit | 2K stars | LSP, formatter, not YAML/JSON |
| **check-jsonschema** | Python | JSON Schema CLI | 48 releases | HTTP caching, pre-commit hooks |
| **graphql-lint** | Rust | GraphQL SDL linter | 11K total | Naming conventions, directive validation |
| `jtd` | Rust | JTD RFC 8927 | 604/month | Official by RFC author, stable but inactive |

## Unique Positioning

**No existing tool** that:
1. Uses GraphQL SDL as schema language
2. Compiles to JTD-like IR for validation
3. Validates both YAML and JSON
4. Has strict/open modes like kubeconform
5. Provides miette-based diagnostics
6. Supports custom schema registries with caching
7. Is general-purpose (not Kubernetes-specific)

## References

- kubeconform repo: https://github.com/yannh/kubeconform
- miette docs: https://docs.rs/miette/latest/miette/
- jsonschema crate: https://crates.io/crates/jsonschema
- serde_valid: https://crates.io/crates/serde_valid
