# Schema Registry Design

## Overview

This document specifies the schema registry subsystem for the GraphQL-ish schema validator. The registry supports local and remote schema sources, composite lookup strategies, and multi-level caching (disk and memory) for performance and reliability.

**Key characteristics:**
- **Multi-source**: Local filesystem and HTTP(S) schema locations
- **Composite**: Try multiple registries in order until schema found
- **Cached**: Disk cache for remote schemas, memory cache for compiled IR
- **Kubeconform-inspired**: Schema location templates, timeout controls, allowlist support
- **Version-based**: Cache invalidation on version changes

## Registry Trait Definition

### Core Registry Interface

```rust
use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait Registry: Send + Sync {
    /// Resolve a schema by ID and version
    async fn resolve(
        &self,
        schema_id: &str,
        version: &str,
    ) -> Result<String, RegistryError>;

    /// Get the underlying schema content (SDL or IR)
    fn get_content(&self, schema_id: &str, version: &str) -> Result<String, RegistryError>;
}

#[derive(Debug, Clone)]
pub struct RegistryError {
    pub code: RegistryErrorCode,
    pub message: String,
    pub schema_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryErrorCode {
    /// Schema not found in any registry
    NotFound,

    /// Network error fetching remote schema
    NetworkError,

    /// Timeout fetching remote schema
    Timeout,

    /// Schema download exceeded size limit
    SizeExceeded,

    /// Failed to parse downloaded schema
    ParseError,

    /// TLS verification failed
    TlsError,

    /// Domain not in allowlist
    DomainNotAllowed,
}
```

**Trait semantics:**
- `resolve`: Async fetch (supports HTTP)
- `get_content`: Blocking fallback for local sources
- Error includes schema ID and version for context

## Local Registry Implementation

### LocalRegistry

```rust
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LocalRegistry {
    base_path: PathBuf,
}

impl LocalRegistry {
    /// Create a new local registry from a base directory
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Get the filesystem path for a schema
    fn schema_path(&self, schema_id: &str, version: &str) -> PathBuf {
        self.base_path
            .join(schema_id)
            .join(version)
            .join("schema.graphql")
    }
}

#[async_trait]
impl Registry for LocalRegistry {
    async fn resolve(
        &self,
        schema_id: &str,
        version: &str,
    ) -> Result<String, RegistryError> {
        let path = self.schema_path(schema_id, version);

        if !path.exists() {
            return Err(RegistryError {
                code: RegistryErrorCode::NotFound,
                message: format!("Schema file not found: {}", path.display()),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            });
        }

        tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| RegistryError {
                code: RegistryErrorCode::ParseError,
                message: format!("Failed to read schema file: {}", e),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            })
    }

    fn get_content(
        &self,
        schema_id: &str,
        version: &str,
    ) -> Result<String, RegistryError> {
        let path = self.schema_path(schema_id, version);

        if !path.exists() {
            return Err(RegistryError {
                code: RegistryErrorCode::NotFound,
                message: format!("Schema file not found: {}", path.display()),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            });
        }

        std::fs::read_to_string(&path).map_err(|e| RegistryError {
            code: RegistryErrorCode::ParseError,
            message: format!("Failed to read schema file: {}", e),
            schema_id: schema_id.to_string(),
            version: version.to_string(),
        })
    }
}
```

**Directory layout:**

```
schemas/
├── unified-workflow/
│   ├── 2.0.0/
│   │   └── schema.graphql
│   └── 2.1.0/
│       └── schema.graphql
├── other-schema/
│   └── 1.0.0/
│       └── schema.graphql
```

**File naming conventions:**
- Schema ID maps to directory name
- Version maps to subdirectory
- Schema file always named `schema.graphql`

## HTTP Registry Implementation

### HttpRegistry

```rust
use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone)]
pub struct HttpRegistry {
    client: Client,
    url_template: String,
    timeout: Duration,
    max_size: usize,
    domain_allowlist: Option<Vec<String>>,
    skip_tls_verify: bool,
    cache: DiskCache,
}

impl HttpRegistry {
    /// Create a new HTTP registry with default settings
    pub fn new(url_template: &str) -> Self {
        Self {
            client: ClientBuilder::new()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            url_template: url_template.to_string(),
            timeout: Duration::from_secs(30),
            max_size: 1024 * 1024, // 1MB default
            domain_allowlist: None,
            skip_tls_verify: false,
            cache: DiskCache::default(),
        }
    }

    /// Set the timeout for HTTP requests
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.client = ClientBuilder::new()
            .timeout(timeout)
            .danger_accept_invalid_certs(self.skip_tls_verify)
            .build()
            .expect("Failed to build HTTP client");
        self
    }

    /// Set the maximum download size
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    /// Set the domain allowlist (None = any domain)
    pub fn with_domain_allowlist(mut self, domains: Vec<String>) -> Self {
        self.domain_allowlist = Some(domains);
        self
    }

    /// Skip TLS verification (insecure, use with caution)
    pub fn with_skip_tls_verify(mut self, skip: bool) -> Self {
        self.skip_tls_verify = skip;
        self.client = ClientBuilder::new()
            .timeout(self.timeout)
            .danger_accept_invalid_certs(skip)
            .build()
            .expect("Failed to build HTTP client");
        self
    }

    /// Build the URL for a schema
    fn build_url(&self, schema_id: &str, version: &str) -> Result<String, RegistryError> {
        let url = self
            .url_template
            .replace("{schema_id}", schema_id)
            .replace("{schema_version}", version)
            .replace("{version}", version);

        // Validate URL
        if let Err(e) = Url::parse(&url) {
            return Err(RegistryError {
                code: RegistryErrorCode::ParseError,
                message: format!("Invalid URL: {}", e),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            });
        }

        Ok(url)
    }

    /// Check if domain is in allowlist
    fn check_domain(&self, url: &str) -> Result<(), RegistryError> {
        if let Some(ref allowlist) = self.domain_allowlist {
            if let Ok(parsed) = Url::parse(url) {
                let domain = parsed.host_str().unwrap_or("");
                if !allowlist.iter().any(|d| domain.ends_with(d)) {
                    return Err(RegistryError {
                        code: RegistryErrorCode::DomainNotAllowed,
                        message: format!("Domain '{}' not in allowlist", domain),
                        schema_id: String::new(),
                        version: String::new(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Registry for HttpRegistry {
    async fn resolve(
        &self,
        schema_id: &str,
        version: &str,
    ) -> Result<String, RegistryError> {
        // Check disk cache first
        if let Ok(cached) = self.cache.get(schema_id, version) {
            return Ok(cached);
        }

        // Build URL
        let url = self.build_url(schema_id, version)?;

        // Check domain allowlist
        self.check_domain(&url)?;

        // Fetch schema
        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| RegistryError {
                code: RegistryErrorCode::NetworkError,
                message: format!("Failed to fetch schema: {}", e),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            })?;

        // Check response status
        if !response.status().is_success() {
            return Err(RegistryError {
                code: RegistryErrorCode::NotFound,
                message: format!("Schema not found: HTTP {}", response.status()),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            });
        }

        // Check content length
        let content_length = response.content_length().unwrap_or(0);
        if content_length > self.max_size as u64 {
            return Err(RegistryError {
                code: RegistryErrorCode::SizeExceeded,
                message: format!(
                    "Schema size {} exceeds limit {}",
                    content_length, self.max_size
                ),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            });
        }

        // Read response body with size limit
        let content = response
            .bytes()
            .await
            .map_err(|e| RegistryError {
                code: RegistryErrorCode::NetworkError,
                message: format!("Failed to read response: {}", e),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            })?
            .to_vec();

        if content.len() > self.max_size {
            return Err(RegistryError {
                code: RegistryErrorCode::SizeExceeded,
                message: format!(
                    "Schema size {} exceeds limit {}",
                    content.len(),
                    self.max_size
                ),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            });
        }

        // Convert to string
        let schema_content = String::from_utf8(content).map_err(|e| RegistryError {
            code: RegistryErrorCode::ParseError,
            message: format!("Failed to decode schema: {}", e),
            schema_id: schema_id.to_string(),
            version: version.to_string(),
        })?;

        // Cache the schema
        self.cache.set(schema_id, version, &schema_content);

        Ok(schema_content)
    }

    fn get_content(
        &self,
        schema_id: &str,
        version: &str,
    ) -> Result<String, RegistryError> {
        // HTTP registry requires async; block on it
        let rt = tokio::runtime::Runtime::new().map_err(|e| RegistryError {
            code: RegistryErrorCode::NetworkError,
            message: format!("Failed to create runtime: {}", e),
            schema_id: schema_id.to_string(),
            version: version.to_string(),
        })?;

        rt.block_on(self.resolve(schema_id, version))
    }
}
```

**Default configuration:**
- Timeout: 30 seconds
- Max size: 1MB
- TLS verification: enabled
- Domain allowlist: none (allow any domain)

**Template variables:**
- `{schema_id}`: Replaced with schema ID (e.g., `unified-workflow`)
- `{schema_version}`: Replaced with version (e.g., `2.0.0`)
- `{version}`: Alias for `{schema_version}`

**Example templates:**

```rust
"https://schemas.example.com/{schema_id}/{schema_version}/schema.graphql"
"https://raw.githubusercontent.com/my-org/schemas/main/{schema_id}/{version}/schema.graphql"
"https://cdn.schemas.io/{schema_id}-{version}.graphql"
```

## Composite Registry Implementation

### CompositeRegistry

```rust
#[derive(Debug, Clone)]
pub struct CompositeRegistry {
    registries: Vec<Box<dyn Registry>>,
}

impl CompositeRegistry {
    /// Create a new composite registry from multiple registries
    pub fn new(registries: Vec<Box<dyn Registry>>) -> Self {
        Self { registries }
    }

    /// Add a registry to the chain
    pub fn add_registry(mut self, registry: Box<dyn Registry>) -> Self {
        self.registries.push(registry);
        self
    }
}

#[async_trait]
impl Registry for CompositeRegistry {
    async fn resolve(
        &self,
        schema_id: &str,
        version: &str,
    ) -> Result<String, RegistryError> {
        let mut last_error = None;

        // Try each registry in order
        for registry in &self.registries {
            match registry.resolve(schema_id, version).await {
                Ok(schema) => return Ok(schema),
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        // All registries failed
        Err(last_error.unwrap_or_else(|| RegistryError {
            code: RegistryErrorCode::NotFound,
            message: "No registries configured".into(),
            schema_id: schema_id.to_string(),
            version: version.to_string(),
        }))
    }

    fn get_content(
        &self,
        schema_id: &str,
        version: &str,
    ) -> Result<String, RegistryError> {
        let mut last_error = None;

        for registry in &self.registries {
            match registry.get_content(schema_id, version) {
                Ok(schema) => return Ok(schema),
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| RegistryError {
            code: RegistryErrorCode::NotFound,
            message: "No registries configured".into(),
            schema_id: schema_id.to_string(),
            version: version.to_string(),
        }))
    }
}
```

**Lookup semantics:**
- Try registries in configured order
- Return first successful result
- Return last error if all fail
- Cache is per-registry (not shared)

## Caching Strategy

### Disk Cache

```rust
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use std::io;

#[derive(Debug, Clone)]
pub struct DiskCache {
    cache_dir: PathBuf,
    max_size: usize, // bytes
}

impl DiskCache {
    /// Create a new disk cache
    pub fn new<P: AsRef<Path>>(cache_dir: P, max_size: usize) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            max_size,
        }
    }

    /// Get cached schema by ID and version
    pub fn get(&self, schema_id: &str, version: &str) -> Result<String, RegistryError> {
        let cache_key = self.cache_key(schema_id, version);
        let cache_path = self.cache_dir.join(&cache_key);

        if !cache_path.exists() {
            return Err(RegistryError {
                code: RegistryErrorCode::NotFound,
                message: "Schema not in cache".into(),
                schema_id: schema_id.to_string(),
                version: version.to_string(),
            });
        }

        fs::read_to_string(&cache_path).map_err(|e| RegistryError {
            code: RegistryErrorCode::ParseError,
            message: format!("Failed to read cache: {}", e),
            schema_id: schema_id.to_string(),
            version: version.to_string(),
        })
    }

    /// Set cached schema by ID and version
    pub fn set(&self, schema_id: &str, version: &str, content: &str) -> io::Result<()> {
        // Ensure cache directory exists
        fs::create_dir_all(&self.cache_dir)?;

        let cache_key = self.cache_key(schema_id, version);
        let cache_path = self.cache_dir.join(&cache_key);

        // Write cache file
        fs::write(&cache_path, content)?;

        // Enforce size limit
        self.enforce_size_limit();

        Ok(())
    }

    /// Clear the entire cache
    pub fn clear(&self) -> io::Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    /// Generate cache key from schema ID and version
    fn cache_key(&self, schema_id: &str, version: &str) -> String {
        // Hash (schema_id, version) to create unique filename
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        format!("{}:{}", schema_id, version).hash(&mut hasher);
        let hash = hasher.finish();

        format!("{:016x}.graphql", hash)
    }

    /// Enforce cache size limit by removing oldest entries
    fn enforce_size_limit(&self) {
        if !self.cache_dir.exists() {
            return;
        }

        let mut entries: Vec<_> = fs::read_dir(&self.cache_dir)
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        e.metadata().ok().and_then(|m| {
                            Some((e.path(), m.modified().ok()?))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        entries.sort_by(|a, b| a.1.cmp(&b.1));

        // Calculate current size
        let current_size: u64 = entries
            .iter()
            .filter_map(|(path, _)| fs::metadata(path).ok())
            .map(|m| m.len())
            .sum();

        if current_size as usize > self.max_size {
            // Remove oldest entries until under limit
            let mut accumulated_size = 0;
            for (path, _) in entries {
                if accumulated_size < self.max_size as u64 {
                    let _ = fs::remove_file(&path);
                    accumulated_size += fs::metadata(&path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                }
            }
        }
    }
}

impl Default for DiskCache {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("graphql-ish-validator");

        Self::new(cache_dir, 100 * 1024 * 1024) // 100MB default
    }
}
```

**Cache key generation:**
- Hash `(schema_id, version)` pair
- Use 16-character hex hash as filename
- Extension `.graphql` for clarity

**Size limit:**
- Default: 100MB
- Enforced by removing oldest entries
- Based on file modification time

**Default cache location:**
- Linux: `~/.cache/graphql-ish-validator/`
- macOS: `~/Library/Caches/graphql-ish-validator/`
- Windows: `%LOCALAPPDATA%\graphql-ish-validator\`

### Memory Cache

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
pub struct MemoryCache {
    cache: LruCache<CacheKey, SchemaBundle>,
    max_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    schema_id: String,
    version: String,
}

impl MemoryCache {
    /// Create a new memory cache with max entries
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(max_entries).unwrap()),
            max_entries,
        }
    }

    /// Get cached compiled IR bundle
    pub fn get(&mut self, schema_id: &str, version: &str) -> Option<&SchemaBundle> {
        let key = CacheKey {
            schema_id: schema_id.to_string(),
            version: version.to_string(),
        };
        self.cache.get(&key)
    }

    /// Set cached compiled IR bundle
    pub fn set(&mut self, schema_id: &str, version: &str, bundle: SchemaBundle) {
        let key = CacheKey {
            schema_id: schema_id.to_string(),
            version: version.to_string(),
        };
        self.cache.put(key, bundle);
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
```

**Cache semantics:**
- LRU (Least Recently Used) eviction
- Default: 100 entries
- Stores compiled IR bundles (not raw SDL)
- Faster than disk cache for repeated lookups

## Cache Invalidation Strategy

### Version-Based Invalidation

```rust
impl CacheInvalidator for DiskCache {
    fn invalidate(&self, schema_id: &str, version: &str) -> io::Result<()> {
        let cache_key = self.cache_key(schema_id, version);
        let cache_path = self.cache_dir.join(&cache_key);

        if cache_path.exists() {
            fs::remove_file(&cache_path)?;
        }

        Ok(())
    }

    fn invalidate_schema_id(&self, schema_id: &str) -> io::Result<()> {
        // Remove all cache entries for a schema ID
        // This requires tracking metadata (schema_id, version) in cache
        // For now, skip this (requires on-disk index)
        Ok(())
    }
}
```

**Invalidation triggers:**
- Version change: Old version cached, new version requested → cache miss
- Explicit invalidation: User runs `--clear-cache` flag
- Time-based: Optional TTL (not implemented initially)

**Cache hit logic:**

```rust
async fn get_or_fetch(
    registry: &dyn Registry,
    memory_cache: &mut MemoryCache,
    schema_id: &str,
    version: &str,
) -> Result<SchemaBundle, RegistryError> {
    // Check memory cache first
    if let Some(bundle) = memory_cache.get(schema_id, version) {
        return Ok(bundle.clone());
    }

    // Fetch from registry (checks disk cache internally)
    let sdl = registry.resolve(schema_id, version).await?;

    // Compile to IR
    let bundle = compile_sdl_to_ir(&sdl)?;

    // Store in memory cache
    memory_cache.set(schema_id, version, bundle.clone());

    Ok(bundle)
}
```

## Schema Discovery from Documents

### Discovery Fields

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SchemaDiscovery {
    /// Schema identifier (e.g., "unified-workflow")
    pub schema_id: String,

    /// Schema version (e.g., "2.0.0")
    pub schema_version: String,
}

impl SchemaDiscovery {
    /// Extract schema metadata from YAML/JSON document
    pub fn from_document(value: &Value) -> Option<Self> {
        match value {
            Value::Object(map) => {
                let schema_id = map.get("schema_id")
                    .or_else(|| map.get("schema_id"))
                    .and_then(|v| v.as_str())
                    .map(String::from)?;

                let schema_version = map.get("schema_version")
                    .or_else(|| map.get("version"))
                    .and_then(|v| v.as_str())
                    .map(String::from)?;

                Some(Self {
                    schema_id,
                    schema_version,
                })
            }
            _ => None,
        }
    }
}
```

**Discovery field precedence:**
- `schema_id`: Direct field
- `schema_version`: Direct field, fallback to `version`

**Example document:**

```yaml
schema_id: unified-workflow
schema_version: 2.0.0
min_schema_version: 2.0.0
```

## Error Handling for Registry Failures

### Error Propagation

```rust
pub async fn resolve_schema(
    registry: &dyn Registry,
    schema_id: &str,
    version: &str,
) -> Result<SchemaBundle, ValidationError> {
    match registry.resolve(schema_id, version).await {
        Ok(sdl) => {
            // Try to compile
            compile_sdl_to_ir(&sdl)
        }
        Err(registry_error) => {
            // Convert registry error to validation error
            Err(ValidationError {
                code: match registry_error.code {
                    RegistryErrorCode::NotFound => ErrorCode::RefUnresolved,
                    RegistryErrorCode::NetworkError => ErrorCode::RefUnresolved,
                    _ => ErrorCode::RefUnresolved,
                },
                instance_path: JsonPointer::new(),
                schema_path: JsonPointer::new(),
                message: format!(
                    "Failed to resolve schema '{}': {}",
                    schema_id, registry_error.message
                ),
                hint: Some("Check registry configuration and network connectivity".into()),
            })
        }
    }
}
```

**Error mapping:**
- `NotFound` → `RefUnresolved`
- `NetworkError` → `RefUnresolved`
- `Timeout` → `RefUnresolved`
- `SizeExceeded` → `RefUnresolved`
- `ParseError` → `RefUnresolved`

**Retry strategy:**
- Network errors: Retry 3 times with exponential backoff
- Timeout errors: Retry 2 times with doubled timeout
- Not found: No retry (permanent failure)

## Cross-References

- **[00-overview.md](./00-overview.md)**: Architecture overview showing registry component
- **[04-validator-runtime.md](./04-validator-runtime.md)**: Registry consumption during `Ref` validation
- **[07-cli-design.md](./07-cli-design.md)**: CLI flags for registry configuration (`--schema-location`, `--cache`, `--timeout`)

## Open Questions and Decisions Needed

1. **Cache metadata storage**: Should we store an on-disk index mapping cache files to `(schema_id, version)` to enable `invalidate_schema_id` operations?

2. **TTL support**: Should we implement time-based cache expiration? If so, what's the default TTL (24h? 7 days?)?

3. **Concurrent cache access**: Should disk cache be thread-safe with mutex/lock, or assume single-process CLI usage?

4. **Composite registry caching**: Should each registry in the composite chain have its own cache, or share a common cache?

5. **Schema discovery fallback**: What if the document doesn't contain `schema_id` and `schema_version` fields? Should we CLI-override or require explicit flags?

## Gap Fix: Schema Versioning Convention

Schema documents can include version information via conventions for registry matching.

### Versioning Convention

SDL schemas can declare version using:

1. **Top-level comment** (recommended):
```graphql
# version: 1.0.0
# x-schema-id: unified-workflow
# x-schema-version: 2.0.0

input WorkflowDocument {
  # ...
}
```

2. **Document metadata** (for YAML/JSON):
```yaml
x-schema-id: unified-workflow
x-schema-version: 2.0.0

workflow_id: abc123
# ...
```

### Registry Matching

Registry matches schemas by `id + version` pair:

```rust
pub struct SchemaKey {
    pub schema_id: String,
    pub schema_version: String,
}

impl SchemaKey {
    fn from_document(value: &Value) -> Option<Self> {
        match value {
            Value::Object(map) => {
                let schema_id = map.get("x-schema-id")
                    .or_else(|| map.get("schema_id"))
                    .and_then(|v| v.as_str())
                    .map(String::from)?;

                let schema_version = map.get("x-schema-version")
                    .or_else(|| map.get("schema_version"))
                    .and_then(|v| v.as_str())
                    .map(String::from)?;

                Some(Self { schema_id, schema_version })
            }
            _ => None,
        }
    }
}
```

### Version Formats

Support semantic versioning:
- `1.0.0` - Major.Minor.Patch
- `2.1.3` - Major.Minor.Patch

Registry treats version as opaque string (no parsing required).

### SDL Comment Parsing

```rust
fn extract_version_from_sdl(sdl: &str) -> Option<(String, String)> {
    let mut schema_id = None;
    let mut schema_version = None;

    for line in sdl.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            // Parse comment
            let content = trimmed[2..].trim();
            if content.starts_with("x-schema-id:") {
                schema_id = Some(content["x-schema-id:".len()..].trim().to_string());
            } else if content.starts_with("x-schema-version:") {
                schema_version = Some(content["x-schema-version:".len()..].trim().to_string());
            }
        }
    }

    match (schema_id, schema_version) {
        (Some(id), Some(ver)) => Some((id, ver)),
        _ => None,
    }
}
```

## Research Links

This design draws from the following ChatGPT research sections:

- **"Schema registries, caching, and kubeconform-inspired conventions"** (second report, lines 607-663): Registry goals, local/HTTP/composite registries, disk caching, HTTP client configuration
- **"kubeconform registry behavior"** (second report, lines 618-622): Multiple schema locations, templated URLs, cache directory
- **"HTTP client and safety configuration"** (second report, lines 647-657): Timeout, max size, domain allowlist, TLS verification
- **"Cache choices"** (second report, lines 658-663): LRU cache, `lru` crate, `moka` for async caching
- **"Schema discovery convention"** (second report, lines 624-645): Schema version fields, template-based lookup
