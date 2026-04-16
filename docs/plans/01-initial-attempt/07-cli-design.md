# CLI Design

## Overview

This document specifies the command-line interface for the GraphQL-ish schema validator. The CLI provides subcommands for validation, compilation, export, and schema checking, with support for both interactive use and CI/CD integration.

**Key characteristics:**
- **Subcommand-based**: Organized by task (`validate`, `compile`, `export-jtd`, `check-schema`)
- **Kubeconform-inspired**: Strict mode flags, schema location templates, caching options
- **Multi-format**: JSON, text, and GitHub Actions output formats
- **Performance-aware**: Shows validation time and document counts
- **CI-friendly**: Exit codes, GitHub Actions format, stdin support

## Binary and Naming

### Project Name Selection

Based on the research report's recommendations, the CLI binary should be named:

**Primary choice**: `gqlsdl-jtd`
- Clear GraphQL SDL heritage
- JTD-like IR positioning
- Short and memorable
- No SDL naming collisions

**Alternative choices:**
- `gqlsdl-validate` (emphasizes validation)
- `gqlsdl-validator` (user-facing)
- `docketry` (abstract brand, requires README explanation)

**Installation:**

```bash
# Install via cargo
cargo install gqlsdl-jtd

# Or build from source
cargo build --release
```

**Usage examples:**

```bash
gqlsdl-jtd validate workflow.yml
gqlsdl-jtd compile schema.graphql --output schema.json
gqlsdl-jtd check-schema schema.graphql
```

## Subcommands

### validate <file-or-dir>

Validate YAML/JSON documents against a schema.

**Synopsis:**

```bash
gqlsdl-jtd validate [OPTIONS] <INPUT>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<INPUT>` | File or directory to validate. Directories are searched recursively. |
| `-s, --schema <FILE_OR_URI>` | Schema to validate against (file path or HTTP URI). |
| `--schema-location <TEMPLATE>` | Schema location template (e.g., `https://schemas.example.com/{schema_id}/{version}/schema.graphql`). Can be specified multiple times. |

**Flags:**

| Flag | Description | Default |
|------|-------------|----------|
| `--strict` | Enable strict mode (reject unknown keys, duplicate keys). | `false` |
| `--open` | Enable open mode (allow unknown keys). | `false` |
| `--format <FORMAT>` | Output format: `json`, `text`, `github-actions`. | `text` |
| `--cache <DIR>` | Cache directory for remote schemas. | `~/.cache/graphql-ish-validator/` |
| `--timeout <DURATION>` | HTTP timeout for remote schemas (e.g., `30s`, `1m`). | `30s` |
| `--no-color` | Disable colored output. | `false` |
| `--verbose` | Enable verbose output. | `false` |
| `-q, --quiet` | Suppress non-error output. | `false` |

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | Valid (all documents passed validation). |
| `1` | Validation errors (at least one document failed validation). |
| `2` | System error (file not found, invalid schema, network failure, etc.). |

**Examples:**

```bash
# Validate a single file against local schema
gqlsdl-jtd validate workflow.yml --schema schema.graphql

# Validate a directory with strict mode
gqlsdl-jtd validate ./workflows --schema schema.graphql --strict

# Validate with remote schema (HTTP)
gqlsdl-jtd validate workflow.yml --schema https://schemas.example.com/unified-workflow/2.0.0/schema.graphql

# Use schema location template (extract schema_id/version from document)
gqlsdl-jtd validate workflow.yml \
  --schema-location "https://schemas.example.com/{schema_id}/{schema_version}/schema.graphql"

# JSON output for tooling
gqlsdl-jtd validate workflow.yml --schema schema.graphql --format json --output errors.json

# GitHub Actions format for CI
gqlsdl-jtd validate ./workflows --schema schema.graphql --format github-actions

# Open mode (allow unknown keys)
gqlsdl-jtd validate workflow.yml --schema schema.graphql --open

# Verbose output with timing
gqlsdl-jtd validate ./workflows --schema schema.graphql --verbose
```

### compile <schema-file>

Compile an SDL schema to IR and output as JSON.

**Synopsis:**

```bash
gqlsdl-jtd compile [OPTIONS] <SCHEMA_FILE>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<SCHEMA_FILE>` | SDL schema file to compile. |

**Flags:**

| Flag | Description | Default |
|------|-------------|----------|
| `-o, --output <FILE>` | Output file for compiled IR JSON. | stdout |
| `--format <FORMAT>` | Output format: `json`, `pretty`. | `json` |
| `--validate` | Validate schema internal consistency before compilation. | `true` |

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | Compilation successful. |
| `1` | Compilation failed (schema errors). |
| `2` | System error (file not found, etc.). |

**Examples:**

```bash
# Compile schema to JSON
gqlsdl-jtd compile schema.graphql --output schema-ir.json

# Compile with pretty-printed JSON
gqlsdl-jtd compile schema.graphql --format pretty --output schema-ir.json

# Compile without validation (faster, risky)
gqlsdl-jtd compile schema.graphql --no-validate
```

### export-jtd <schema-file>

Export a compiled IR schema to JTD JSON format (where representable).

**Synopsis:**

```bash
gqlsdl-jtd export-jtd [OPTIONS] <SCHEMA_FILE>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<SCHEMA_FILE>` | SDL schema file to export. |

**Flags:**

| Flag | Description | Default |
|------|-------------|----------|
| `-o, --output <FILE>` | Output file for JTD JSON. | stdout |
| `--strict-jtd` | Fail on features not representable in JTD. | `false` |
| `--allow-extensions` | Allow JTD extensions in output (non-standard). | `false` |

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | Export successful. |
| `1` | Export failed (schema uses non-JTD features). |
| `2` | System error (file not found, etc.). |

**Examples:**

```bash
# Export schema to JTD
gqlsdl-jtd export-jtd schema.graphql --output schema.jtd.json

# Strict mode (fail if schema uses @mapRest)
gqlsdl-jtd export-jtd schema.graphql --strict-jtd

# Allow extensions (emit non-standard JTD features)
gqlsdl-jtd export-jtd schema.graphql --allow-extensions
```

**JTD representation notes:**

| IR feature | JTD equivalent | Exportable? |
|------------|----------------|--------------|
| `Scalar` | `type` form | Yes (basic types only) |
| `Enum` | `enum` form | Yes |
| `Array` | `elements` form | Yes |
| `Object` | `properties` form | Yes (additional properties limited) |
| `Map` | `values` form | Yes |
| `DiscriminatedUnion` | `discriminator` form | Yes |
| `OneOf` | Not in JTD | No (requires custom `oneOf` extension) |
| `@mapRest` | Not in JTD | No (requires `additionalProperties` with schema extension) |

### check-schema <schema-file>

Validate SDL schema for internal consistency without compiling.

**Synopsis:**

```bash
gqlsdl-jtd check-schema [OPTIONS] <SCHEMA_FILE>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<SCHEMA_FILE>` | SDL schema file to validate. |

**Flags:**

| Flag | Description | Default |
|------|-------------|----------|
| `--format <FORMAT>` | Output format: `json`, `text`. | `text` |
| `--no-color` | Disable colored output. | `false` |

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | Schema is valid. |
| `1` | Schema has errors. |
| `2` | System error (file not found, etc.). |

**Checks performed:**

- Unknown directives
- Duplicate type names
- Undefined type references
- Circular type definitions (without intermediate types)
- Union coherence (discriminator mapping consistency)
- Default value type compatibility
- Regex pattern validity

**Examples:**

```bash
# Check schema
gqlsdl-jtd check-schema schema.graphql

# JSON output for tooling
gqlsdl-jtd check-schema schema.graphql --format json
```

## Detailed Flag Specifications

### --schema <FILE_OR_URI>

**Purpose**: Specify the schema to validate against.

**Behavior:**
- If path starts with `http://` or `https://`, fetch via HTTP
- If path starts with `/` or `./`, treat as local file path
- Otherwise, resolve relative to current directory

**Priority:**
1. `--schema` flag (explicit)
2. `--schema-location` templates (extract from document)
3. Default schema location (if configured)

### --schema-location <TEMPLATE>

**Purpose**: Define schema location template for automatic discovery.

## Gap Fix: Schema Location Template Syntax

Template-based schema location URLs use simple placeholder syntax without template engine dependency.

### Template Syntax

**Placeholders**:
- `{schema_id}` - Schema identifier from document
- `{schema_version}` - Schema version from document
- `{version}` - Alias for `{schema_version}`

**No template engine**: Simple `str::replace` operations only.

### Implementation

```rust
fn expand_template(template: &str, schema_id: &str, version: &str) -> String {
    template
        .replace("{schema_id}", schema_id)
        .replace("{schema_version}", version)
        .replace("{version}", version)
}

// Example
let template = "https://example.com/schemas/{schema_id}/v{schema_version}/schema.graphql";
let url = expand_template(template, "unified-workflow", "2.0.0");
// Result: "https://example.com/schemas/unified-workflow/v2.0.0/schema.graphql"
```

### Template Variables

| Placeholder | Source | Example |
|-------------|---------|---------|
| `{schema_id}` | `schema_id` field in YAML/JSON document | `unified-workflow` |
| `{schema_version}` | `schema_version` or `version` field in document | `2.0.0` |
| `{version}` | Alias for `{schema_version}` | `2.0.0` |

### Example Templates

```bash
# GitHub raw content
--schema-location "https://raw.githubusercontent.com/my-org/schemas/main/{schema_id}/{schema_version}/schema.graphql"

# Custom schema server
--schema-location "https://schemas.example.com/{schema_id}/{schema_version}/schema.graphql"

# Local filesystem template
--schema-location "file:///abs/path/schemas/{schema_id}/{schema_version}/schema.graphql"

# Multiple fallback locations
--schema-location "https://primary.example.com/{schema_id}/{schema_version}/schema.graphql" \
  --schema-location "https://fallback.example.com/{schema_id}/{schema_version}/schema.graphql"
```

**Behavior:**
- Multiple `--schema-location` flags are tried in order
- First successful fetch is used
- Template expansion uses fields from document being validated

### --strict vs --open

**Purpose**: Control strictness behavior for validation.

**Strict mode:**
- Reject additional keys (unless `@mapRest` explicitly allows)
- Reject duplicate YAML keys at parse time
- No type coercion (`"123"` stays string)
- Enforce `@closed` directive on all objects
- Unknown directives in schema are errors

**Open mode:**
- Allow additional keys (ignore them)
- Accept duplicate YAML keys (last wins)
- Allow limited type coercion (YAML `1` → Float scalar)
- Ignore `@closed` directive
- Unknown directives are warnings

**Default behavior:**
- Neither `--strict` nor `--open` → schema defaults
- `@closed` directive → reject additional keys
- No `@closed` → allow additional keys

### --format <json|text|github-actions>

**Purpose**: Select output format.

**Formats:**

| Format | Description | Use case |
|---------|-------------|-----------|
| `json` | Machine-readable JSON array of error objects. | Tooling, CI, programmatic use. |
| `text` | Human-readable multi-line with colors. | Interactive use, terminal output. |
| `github-actions` | GitHub Actions workflow command format. | CI/CD integration. |

**JSON format:**

```json
[
  {
    "instancePath": "/agentic_workflow/steps/read_config/input",
    "schemaPath": "/definitions/ToolStep/properties/input",
    "code": "type_mismatch",
    "message": "Expected object, got string",
    "hint": "ToolStep.input must be a map with tool-specific arguments."
  }
]
```

**Text format:**

```
✗ unified-workflow-schema.yml: 3 validation errors

Error 1:
  Code: type_mismatch
  Instance path: /agentic_workflow/steps/read_config/input

    Expected: object
    Found: string

    ToolStep.input must be a map with tool-specific arguments.
```

**GitHub Actions format:**

```yaml
::error file=unified-workflow-schema.yml,line=42,title=type_mismatch::Expected object, got string at /agentic_workflow/steps/read_config/input
```

### --cache <DIR>

**Purpose**: Specify cache directory for remote schemas.

**Default locations:**
- Linux: `~/.cache/graphql-ish-validator/`
- macOS: `~/Library/Caches/graphql-ish-validator/`
- Windows: `%LOCALAPPDATA%\graphql-ish-validator\`

**Behavior:**
- Cache is keyed by `(schema_id, version)` hash
- Cache is version-based (schema version changes invalidate cache)
- Default cache size limit: 100MB
- Use `--cache /dev/null` to disable caching

**Examples:**

```bash
# Use custom cache directory
gqlsdl-jtd validate workflow.yml \
  --schema https://schemas.example.com/schema.graphql \
  --cache /tmp/schema-cache

# Disable caching
gqlsdl-jtd validate workflow.yml \
  --schema https://schemas.example.com/schema.graphql \
  --cache /dev/null
```

### --timeout <DURATION>

**Purpose**: Set HTTP timeout for remote schema fetching.

**Format:**
- Number followed by unit: `s` (seconds), `m` (minutes), `h` (hours)
- Default: `30s`

**Examples:**

```bash
# 10 second timeout
--timeout 10s

# 5 minute timeout
--timeout 5m

# 1 hour timeout
--timeout 1h
```

### --no-color

**Purpose**: Disable colored terminal output.

**Use cases:**
- CI/CD pipelines (color codes clutter logs)
- File redirection (pipes to files)
- Non-terminal output

**Example:**

```bash
gqlsdl-jtd validate workflow.yml --schema schema.graphql --no-color > errors.txt
```

### --verbose

**Purpose**: Enable verbose logging.

**Additional output:**
- Schema resolution details
- Cache hit/miss information
- Compilation progress
- Performance metrics (time, document counts)
- Network requests (URLs, status codes)

**Example:**

```bash
gqlsdl-jtd validate ./workflows --schema schema.graphql --verbose
```

**Verbose output example:**

```
Resolving schema: unified-workflow@2.0.0
  Trying: https://schemas.example.com/unified-workflow/2.0.0/schema.graphql
  Cache miss: fetching from network
  Downloaded: 45KB in 120ms
  Cache stored: ~/.cache/graphql-ish-validator/a1b2c3d4e5f6789.graphql

Compiling SDL to IR...
  Parsed 42 type definitions
  Resolved 15 type references
  Generated IR bundle in 15ms

Validating documents: 3 found
  workflow1.yml: valid (5ms)
  workflow2.yml: 3 errors (8ms)
  workflow3.yml: valid (4ms)

Summary:
  Documents: 3
  Valid: 2
  Invalid: 1
  Total errors: 3
  Time: 152ms
```

### --quiet

**Purpose:** Suppress non-error output.

**Behavior:**
- Only error output is printed
- No success messages, no summary, no progress

**Use case:**
- CI/CD pipelines (only fail on errors)
- Automated scripts

**Example:**

```bash
gqlsdl-jtd validate ./workflows --schema schema.graphql --quiet
# Output: nothing if valid, errors only if invalid
```

## File Discovery

### Directory Traversal

**Behavior:**
- Recursively search directories for files
- Filter by extension: `.yml`, `.yaml`, `.json`
- Follow symbolic links (default)
- Ignore hidden files (`.filename`)

**Examples:**

```bash
# Validate all YAML/JSON in ./workflows
gqlsdl-jtd validate ./workflows --schema schema.graphql

# Validate only .yml files (implicit)
gqlsdl-jtd validate ./workflows --schema schema.graphql
```

**Extension filtering:**

| Extension | Detected as |
|------------|---------------|
| `.yml` | YAML |
| `.yaml` | YAML |
| `.json` | JSON |

### stdin Support

**Purpose:** Pipe YAML/JSON content via stdin.

**Syntax:**

```bash
cat workflow.yml | gqlsdl-jtd validate --schema schema.graphql
echo '{"name": "test"}' | gqlsdl-jtd validate --schema schema.graphql
```

**Behavior:**
- Reads from stdin when no file argument is provided
- Treats stdin content as a single document
- Works with both YAML and JSON

## Performance Reporting

### Summary Statistics

**Always shown (unless --quiet):**

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Summary:
  Documents: 3
  Valid: 2
  Invalid: 1
  Errors: 3
  Strict mode: true
  Time: 152ms
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**Fields:**
- `Documents`: Total number of documents validated
- `Valid`: Number of documents that passed validation
- `Invalid`: Number of documents that failed validation
- `Errors`: Total number of validation errors across all documents
- `Strict mode`: Whether strict mode was enabled
- `Time`: Total validation time (includes schema resolution)

### Performance Metrics (--verbose)

**Additional metrics:**

```
Performance metrics:
  Schema resolution: 120ms
  Compilation: 15ms
  Validation: 17ms (total)
  Average per document: 5.7ms
  Cache hit rate: 0%
  Documents per second: 19.7
```

**Metrics:**
- `Schema resolution`: Time to fetch and compile schema
- `Compilation`: Time to compile SDL to IR (if not cached)
- `Validation`: Time to validate all documents
- `Average per document`: `Validation` / `Documents`
- `Cache hit rate`: Percentage of documents served from cache
- `Documents per second`: `Documents` / `Time`

## Cross-References

- **[00-overview.md](./00-overview.md)**: Architecture overview showing CLI component
- **[04-validator-runtime.md](./04-validator-runtime.md)**: Validation algorithm, strict/open modes
- **[05-error-reporting.md](./05-error-reporting.md)**: Output formats (JSON, text, GitHub Actions)
- **[06-schema-registry.md](./06-schema-registry.md)**: Schema location templates, caching, timeout

## Open Questions and Decisions Needed

1. **Default schema behavior**: If neither `--schema` nor `--schema-location` is provided, should we:
   - Require one (error)?
   - Use a default schema file name (e.g., `schema.graphql` in current dir)?
   - Extract `schema_id`/`schema_version` from document and look up in default registry?

2. **Parallel validation**: Should we validate multiple documents in parallel (rayon) or sequentially? Parallel is faster but makes error output order non-deterministic.

3. **Stdin schema support**: Should we support reading schema from stdin (e.g., `cat schema.graphql | gqlsdl-jtd validate - workflow.yml`)?

4. **Config file support**: Should we support a `.gqlsdl-jtd.toml` or `.gqlsdl-jtd.yaml` config file for default flags?

5. **Tab completion**: Should we generate shell completion scripts (bash, zsh, fish) and include them in the binary?

6. **Output file flags**: Should `validate` command support `--output <FILE>` to write JSON errors to file (instead of `--format json > errors.json`)?

## Research Links

This design draws from the following ChatGPT research sections:

- **"CLI binary and subcommands"** (second report, lines 572-580): Validate, compile, export, check-schema commands
- **"kubeconform-like CLI flags"** (second report, lines 621-622): Schema locations, strict mode, caching
- **"Schema registry behavior"** (second report, lines 618-622): Multiple schema locations, templates
- **"HTTP client configuration"** (second report, lines 647-657): Timeout, max size, domain allowlist, TLS
- **"Validation summary"** (second report, lines 573-579): Document counts, error counts, timing
- **"Repository naming"** (first report, lines 88-99): Name recommendations (`gqlsdl-jtd`, `docketry`)
