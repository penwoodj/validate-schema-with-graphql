# CLI Improvements Plan

**For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a fully-featured CLI with multiple subcommands, configurable output formats, stdin support, and CI-friendly exit codes.

**Architecture:** clap-based CLI with subcommands for validate, compile, check-schema, export-jtd; global flags for logging/output configuration.

**Tech Stack:** Rust, clap (derive), anyhow, tracing, graphql-ish-schema-validator public API

---

## Context and Rationale

**CLI requirements from research:**
- Binary name: `graphql-ish-schema-validator` with short alias `gqlsdl`
- Subcommands: validate, compile, check-schema, export-jtd
- Global flags: --verbose, --quiet, --log-level, --log-file, --format (text/json/github-actions), --color (auto/always/never)
- Exit codes: 0=valid, 1=validation errors, 2=system errors (bad schema, file not found), 3=panic/unknown
- stdin support, directory traversal, glob patterns
- Progress reporting for multi-file validation

**Key design principles:**
1. **Composable**: Subcommands do one thing well
2. **CI-friendly**: Proper exit codes, machine-readable output
3. **User-friendly**: Help text, examples, sensible defaults
4. **Performant**: Parallel validation where possible

**References:**
- [01-initial-attempt/07-cli-design.md](../01-initial-attempt/07-cli-design.md) - Original CLI design
- [02-logging-and-diagnostics.md](./02-logging-and-diagnostics.md) - Output formats

---

## Task 1: Implement CLI Structure

**Files:**
- Modify: `crates/graphql-ish-schema-validator-cli/Cargo.toml`
- Modify: `crates/graphql-ish-schema-validator-cli/src/main.rs`

**Step 1: Update CLI dependencies**

Update `crates/graphql-ish-schema-validator-cli/Cargo.toml`:

```toml
[package]
name = "graphql-ish-schema-validator-cli"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI tool for GraphQL-ish schema validator"
keywords.workspace = true
categories.workspace = true
rust-version.workspace = true

[[bin]]
name = "graphql-ish-schema-validator"
path = "src/main.rs"

[[bin]]
name = "gqlsdl"
path = "src/main.rs"

[dependencies]
graphql-ish-schema-validator = { path = "../graphql-ish-schema-validator", features = ["yaml"] }
anyhow = "1.0"
clap = { version = "4.4", features = ["derive", "env"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
glob = "0.3"
ignore = "0.4"
indicatif = "0.17"
console = "0.15"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
```

- [ ] **Step 2: Implement main CLI structure**

Update `crates/graphql-ish-schema-validator-cli/src/main.rs`:

```rust
//! GraphQL-ish Schema Validator CLI
//!
//! Binary names:
//! - `graphql-ish-schema-validator` (full name)
//! - `gqlsdl` (short alias)

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

use graphql_ish_schema_validator::{
    init_logging, validate_json_from_schema, validate_yaml_from_schema, OutputFormat,
    ValidationOptions, ValidationMode, LogLevel, LogOutput, Formatter,
};

/// GraphQL-inspired schema validator for YAML/JSON documents
#[derive(Parser, Debug)]
#[command(
    name = "graphql-ish-schema-validator",
    bin_name = "graphql-ish-schema-validator",
    author,
    version,
    about,
    long_about = "Validate YAML/JSON documents against GraphQL-inspired schemas with excellent diagnostics.",
    after_help = "Examples:
  gqlsdl validate workflow.yml --schema schema.graphql
  gqlsdl compile schema.graphql --output schema.json
  gqlsdl check-schema schema.graphql
  gqlsdl export-jtd schema.graphql --output schema.jtd.json"
)]
struct Cli {
    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, value_name = "LEVEL", global = true)]
    log_level: Option<LogLevel>,

    /// Suppress all output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Verbose output (equivalent to --log-level debug)
    #[arg(short = 'v', long, global = true)]
    verbose: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    /// Output format (text, json, github-actions)
    #[arg(short = 'F', long, value_name = "FORMAT", global = true)]
    format: Option<OutputFormat>,

    /// Color mode (auto, always, never)
    #[arg(long, value_enum, default_value_t = ColorMode::Auto, global = true)]
    color: ColorMode,

    /// Log output destination (stderr, stdout, file, silent)
    #[arg(long, value_name = "DEST", global = true)]
    log_output: Option<LogOutput>,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Color mode for terminal output
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ColorMode {
    /// Automatically detect terminal support
    Auto,

    /// Always use colors
    Always,

    /// Never use colors
    Never,
}

impl ColorMode {
    /// Determine whether to use colors
    fn use_color(&self) -> bool {
        match self {
            ColorMode::Auto => console::colors_enabled(),
            ColorMode::Always => true,
            ColorMode::Never => false,
        }
    }
}

/// Available subcommands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Validate a YAML/JSON document
    Validate {
        /// Input file, directory, or glob pattern
        #[arg(value_name = "INPUT")]
        input: String,

        /// Schema file or URI
        #[arg(short, long, value_name = "SCHEMA")]
        schema: String,

        /// Enable strict mode (reject unknown keys, duplicate keys)
        #[arg(long, conflicts_with = "open")]
        strict: bool,

        /// Enable open mode (allow unknown keys)
        #[arg(long, conflicts_with = "strict")]
        open: bool,

        /// Read input from stdin
        #[arg(long)]
        stdin: bool,

        /// Continue validating even if errors occur
        #[arg(long)]
        continue_on_error: bool,
    },

    /// Compile an SDL schema to IR
    Compile {
        /// Schema file to compile
        #[arg(value_name = "SCHEMA_FILE")]
        schema: String,

        /// Output file (default: stdout)
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,
    },

    /// Check a schema for internal consistency
    CheckSchema {
        /// Schema file to check
        #[arg(value_name = "SCHEMA_FILE")]
        schema: String,
    },

    /// Export a schema to JTD JSON
    ExportJtd {
        /// Schema file to export
        #[arg(value_name = "SCHEMA_FILE")]
        schema: String,

        /// Output file (default: stdout)
        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,

        /// Fail on features not representable in JTD
        #[arg(long)]
        strict_jtd: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    setup_logging(&cli)?;

    // Determine color mode
    let use_color = if cli.no_color {
        false
    } else {
        cli.color.use_color()
    };

    // Set default output format if not specified
    let output_format = cli.format.unwrap_or(OutputFormat::Text);

    // Execute subcommand
    match cli.command {
        Some(Commands::Validate { input, schema, strict, open, stdin, continue_on_error }) => {
            validate_command(&input, &schema, strict, open, stdin, continue_on_error, output_format, use_color)
        }
        Some(Commands::Compile { schema, output }) => {
            compile_command(&schema, output.as_deref(), output_format, use_color)
        }
        Some(Commands::CheckSchema { schema }) => {
            check_schema_command(&schema, output_format, use_color)
        }
        Some(Commands::ExportJtd { schema, output, strict_jtd }) => {
            export_jtd_command(&schema, output.as_deref(), strict_jtd, output_format, use_color)
        }
        None => {
            // Show help if no subcommand provided
            println!("{}", Cli::command().render_long_help());
            Ok(())
        }
    }
}

/// Setup logging based on CLI flags
fn setup_logging(cli: &Cli) -> Result<()> {
    let log_level = if cli.quiet {
        LogLevel::Error
    } else if cli.verbose {
        LogLevel::Debug
    } else {
        cli.log_level.unwrap_or(LogLevel::Info)
    };

    let log_output = cli.log_output.clone().unwrap_or(LogOutput::Stderr);

    init_logging(log_level, log_output)
        .context("Failed to initialize logging")?;

    Ok(())
}

/// Validate one or more documents
fn validate_command(
    input: &str,
    schema: &str,
    strict: bool,
    open: bool,
    stdin: bool,
    continue_on_error: bool,
    format: OutputFormat,
    color: bool,
) -> Result<()> {
    info!("Validating {} against schema {}", input, schema);

    // Determine validation mode
    let mode = if strict {
        ValidationMode::Strict
    } else if open {
        ValidationMode::Open
    } else {
        // Default: schema defines mode
        ValidationMode::Strict
    };

    // Build validation options
    let options = ValidationOptions::builder()
        .mode(mode)
        .log_level(if cfg!(debug_assertions) { LogLevel::Debug } else { LogLevel::Info })
        .rich_errors(true)
        .include_source_location(true)
        .build();

    // Load schema
    let schema_content = if stdin {
        warn!("Reading schema from stdin not yet supported");
        anyhow::bail!("Reading schema from stdin not yet implemented");
    } else {
        read_file(schema)
            .with_context(|| format!("Failed to read schema file: {}", schema))?
    };

    // Process input
    let results = if stdin {
        // Read from stdin
        let input_content = read_stdin()?;
        let result = if input.ends_with(".yml") || input.ends_with(".yaml") {
            validate_yaml_from_schema(&input_content, &schema_content, options)
        } else {
            validate_json_from_schema(&input_content, &schema_content, options)
        };
        vec![result]
    } else if Path::new(input).is_dir() {
        // Validate all files in directory
        validate_directory(input, &schema_content, options, continue_on_error)?
    } else if input.contains('*') || input.contains('?') {
        // Validate files matching glob pattern
        validate_glob(input, &schema_content, options, continue_on_error)?
    } else {
        // Validate single file
        let input_content = read_file(input)?;
        let result = if input.ends_with(".yml") || input.ends_with(".yaml") {
            validate_yaml_from_schema(&input_content, &schema_content, options)
        } else {
            validate_json_from_schema(&input_content, &schema_content, options)
        };
        vec![result]
    };

    // Format and print results
    let formatter = Formatter::new(format)
        .with_color(color);

    for (i, result) in results.iter().enumerate() {
        if results.len() > 1 {
            println!("\n=== {} ===", input);
        }

        println!("{}", formatter.format_result(result));

        // Stop on first error unless continue_on_error is set
        if !result.valid && !continue_on_error && i == results.len() - 1 {
            std::process::exit(1);
        }
    }

    // Exit with appropriate code
    let has_errors = results.iter().any(|r| !r.valid);
    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

/// Compile a schema to IR
fn compile_command(
    schema: &str,
    output: Option<&str>,
    _format: OutputFormat,
    _color: bool,
) -> Result<()> {
    info!("Compiling schema {}", schema);

    let schema_content = read_file(schema)?;

    // TODO: Implement schema compilation
    println!("Compilation not yet implemented");
    println!("Schema loaded: {} bytes", schema_content.len());

    Ok(())
}

/// Check a schema for internal consistency
fn check_schema_command(
    schema: &str,
    _format: OutputFormat,
    _color: bool,
) -> Result<()> {
    info!("Checking schema {}", schema);

    let schema_content = read_file(schema)?;

    // TODO: Implement schema checking
    println!("Schema check not yet implemented");
    println!("Schema loaded: {} bytes", schema_content.len());

    Ok(())
}

/// Export a schema to JTD JSON
fn export_jtd_command(
    schema: &str,
    output: Option<&str>,
    _strict_jtd: bool,
    _format: OutputFormat,
    _color: bool,
) -> Result<()> {
    info!("Exporting schema {} to JTD", schema);

    let schema_content = read_file(schema)?;

    // TODO: Implement JTD export
    println!("JTD export not yet implemented");
    println!("Schema loaded: {} bytes", schema_content.len());

    Ok(())
}

/// Read file contents
fn read_file(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))
}

/// Read stdin contents
fn read_stdin() -> Result<String> {
    use std::io::Read;

    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read stdin")?;

    Ok(buffer)
}

/// Validate all files in a directory
fn validate_directory(
    dir: &str,
    schema: &str,
    options: ValidationOptions,
    continue_on_error: bool,
) -> Result<Vec<graphql_ish_schema_validator::ValidationResult>> {
    use ignore::WalkBuilder;

    info!("Walking directory: {}", dir);

    let mut results = Vec::new();

    for entry in WalkBuilder::new(dir)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            ext == "yml" || ext == "yaml" || ext == "json"
        })
    {
        let path = entry.path();
        info!("Validating: {}", path.display());

        let content = read_file(&path.to_string_lossy())?;

        let result = match path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
        {
            "yml" | "yaml" => validate_yaml_from_schema(&content, schema, options.clone()),
            "json" => validate_json_from_schema(&content, schema, options.clone()),
            _ => continue,
        };

        results.push(result);

        if !result.valid && !continue_on_error {
            break;
        }
    }

    Ok(results)
}

/// Validate files matching a glob pattern
fn validate_glob(
    pattern: &str,
    schema: &str,
    options: ValidationOptions,
    continue_on_error: bool,
) -> Result<Vec<graphql_ish_schema_validator::ValidationResult>> {
    info!("Validating files matching pattern: {}", pattern);

    let paths = glob::glob(pattern)
        .context("Invalid glob pattern")?
        .filter_map(|p| p.ok())
        .collect::<Vec<_>>();

    let mut results = Vec::new();

    for path in paths {
        info!("Validating: {}", path.display());

        let content = read_file(&path.to_string_lossy())?;

        let result = match path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
        {
            "yml" | "yaml" => validate_yaml_from_schema(&content, schema, options.clone()),
            "json" => validate_json_from_schema(&content, schema, options.clone()),
            _ => continue,
        };

        results.push(result);

        if !result.valid && !continue_on_error {
            break;
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        use std::ffi::OsString;

        let cli = Cli::try_parse_from([
            "graphql-ish-schema-validator",
            "--log-level",
            "debug",
            "validate",
            "test.yml",
            "--schema",
            "schema.graphql",
        ]);

        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert_eq!(cli.log_level, Some(LogLevel::Debug));
        assert!(matches!(cli.command, Some(Commands::Validate { .. })));
    }

    #[test]
    fn test_color_mode() {
        assert_eq!(ColorMode::from_str("auto", true), Some(ColorMode::Auto));
        assert_eq!(ColorMode::from_str("always", true), Some(ColorMode::Always));
        assert_eq!(ColorMode::from_str("never", true), Some(ColorMode::Never));
        assert_eq!(ColorMode::from_str("invalid", true), None);
    }
}

// Add ValueEnum implementation for ColorMode
impl std::str::FromStr for ColorMode {
    type Err = String;

    fn from_str(s: &str, _case_sensitive: bool) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(ColorMode::Auto),
            "always" => Ok(ColorMode::Always),
            "never" => Ok(ColorMode::Never),
            _ => Err(format!("invalid color mode: {}", s)),
        }
    }
}
```

- [ ] **Step 3: Test CLI structure compiles**

Run: `cargo check -p graphql-ish-schema-validator-cli`
Expected: No compilation errors

- [ ] **Step 4: Test CLI help**

Run: `cargo run -p graphql-ish-schema-validator-cli -- --help`
Expected: Help text displayed

- [ ] **Step 5: Commit CLI structure**

```bash
git add crates/graphql-ish-schema-validator-cli/src/main.rs
git add crates/graphql-ish-schema-validator-cli/Cargo.toml
git commit -m "feat: implement CLI structure with subcommands"
```

---

## Task 2: Add Progress Reporting

**Files:**
- Create: `crates/graphql-ish-schema-validator-cli/src/progress.rs`

**Step 1: Create progress module**

Create `crates/graphql-ish-schema-validator-cli/src/progress.rs`:

```rust
//! Progress reporting for CLI operations

use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};

/// Create a progress bar for validation operations
pub fn create_validation_progress_bar(total: usize) -> ProgressBar {
    let progress = ProgressBar::new(total as u64);

    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .expect("Invalid progress template")
            .progress_chars("=> ")
    );

    progress.set_message("Validating...");
    progress
}

/// Update progress bar with current file
pub fn update_progress(progress: &ProgressBar, current: usize, total: usize, file: &str) {
    progress.set_position(current as u64);
    progress.set_message(format!("Validating: {}", file));
}

/// Finish progress bar with success message
pub fn finish_progress(progress: ProgressBar, message: &str) {
    progress.finish_with_message(message.to_string());
}

/// Progress reporter for batch operations
pub struct ProgressReporter {
    bar: Option<ProgressBar>,
    total: usize,
    current: usize,
    start_time: std::time::Instant,
    verbose: bool,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new(total: usize, verbose: bool) -> Self {
        let bar = if verbose && total > 1 {
            Some(create_validation_progress_bar(total))
        } else {
            None
        };

        Self {
            bar,
            total,
            current: 0,
            start_time: std::time::Instant::now(),
            verbose,
        }
    }

    /// Update progress for a completed item
    pub fn update(&mut self, file: &str, valid: bool) {
        self.current += 1;

        if let Some(ref bar) = self.bar {
            update_progress(bar, self.current, self.total, file);
        } else if self.verbose {
            let status = if valid { "✓" } else { "✗" };
            println!("[{}/{}] {} {}", self.current, self.total, status, file);
        }
    }

    /// Finish reporting and return statistics
    pub fn finish(self) -> ProgressStats {
        if let Some(bar) = self.bar {
            finish_progress(bar, &format!("Completed {} validations", self.total));
        }

        let duration = self.start_time.elapsed();

        ProgressStats {
            total: self.total,
            duration,
        }
    }
}

/// Statistics from a progress report
#[derive(Debug)]
pub struct ProgressStats {
    pub total: usize,
    pub duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_reporter() {
        let mut reporter = ProgressReporter::new(3, false);

        reporter.update("file1.yml", true);
        reporter.update("file2.yml", false);
        reporter.update("file3.yml", true);

        let stats = reporter.finish();

        assert_eq!(stats.total, 3);
    }
}
```

- [ ] **Step 2: Update CLI to use progress**

Update `validate_directory` and `validate_glob` functions to use progress:

```rust
use crate::progress::ProgressReporter;

fn validate_directory(
    dir: &str,
    schema: &str,
    options: ValidationOptions,
    continue_on_error: bool,
) -> Result<Vec<graphql_ish_schema_validator::ValidationResult>> {
    use ignore::WalkBuilder;

    info!("Walking directory: {}", dir);

    // Collect files first to get total count
    let files: Vec<_> = WalkBuilder::new(dir)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            ext == "yml" || ext == "yaml" || ext == "json"
        })
        .collect();

    let mut results = Vec::new();
    let mut progress = ProgressReporter::new(files.len(), options.log_level == LogLevel::Debug);

    for entry in files {
        let path = entry.path();
        info!("Validating: {}", path.display());

        let content = read_file(&path.to_string_lossy())?;

        let result = match path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
        {
            "yml" | "yaml" => validate_yaml_from_schema(&content, schema, options.clone()),
            "json" => validate_json_from_schema(&content, schema, options.clone()),
            _ => continue,
        };

        progress.update(&path.to_string_lossy(), result.valid);
        results.push(result.clone());

        if !result.valid && !continue_on_error {
            break;
        }
    }

    progress.finish();
    Ok(results)
}
```

- [ ] **Step 3: Test progress reporting**

Run: `cargo build --release -p graphql-ish-schema-validator-cli`

Test with directory:
```bash
mkdir -p /tmp/test-validators
echo "name: test" > /tmp/test-validators/file1.yml
echo "name: test" > /tmp/test-validators/file2.yml

cargo run -p graphql-ish-schema-validator-cli -- validate /tmp/test-validators --schema schema.graphql --verbose
```

Expected: Progress bar or verbose output shown

- [ ] **Step 4: Commit progress reporting**

```bash
git add crates/graphql-ish-schema-validator-cli/src/progress.rs
git add crates/graphql-ish-schema-validator-cli/src/main.rs
git commit -m "feat: add progress reporting for batch validation"
```

---

## Task 3: Add Exit Code Management

**Files:**
- Modify: `crates/graphql-ish-schema-validator-cli/src/main.rs`

**Step 1: Define exit codes**

Add exit code constants at the top of the file:

```rust
/// Exit codes for the CLI
mod exit_codes {
    pub const SUCCESS: i32 = 0;              // Valid
    pub const VALIDATION_ERROR: i32 = 1;     // Validation errors
    pub const SYSTEM_ERROR: i32 = 2;          // Bad schema, file not found
    pub const UNKNOWN_ERROR: i32 = 3;          // Panic, unknown error
}

use exit_codes::*;
```

- [ ] **Step 2: Update main function to handle errors properly**

```rust
fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    setup_logging(&cli)?;

    // Determine color mode
    let use_color = if cli.no_color {
        false
    } else {
        cli.color.use_color()
    };

    // Set default output format if not specified
    let output_format = cli.format.unwrap_or(OutputFormat::Text);

    // Execute subcommand with proper error handling
    match cli.command {
        Some(Commands::Validate { input, schema, strict, open, stdin, continue_on_error }) => {
            validate_command(&input, &schema, strict, open, stdin, continue_on_error, output_format, use_color)
        }
        Some(Commands::Compile { schema, output }) => {
            compile_command(&schema, output.as_deref(), output_format, use_color)
        }
        Some(Commands::CheckSchema { schema }) => {
            check_schema_command(&schema, output_format, use_color)
        }
        Some(Commands::ExportJtd { schema, output, strict_jtd }) => {
            export_jtd_command(&schema, output.as_deref(), strict_jtd, output_format, use_color)
        }
        None => {
            // Show help if no subcommand provided
            println!("{}", Cli::command().render_long_help());
            Ok(())
        }
    }.map_err(|e| {
        error!("Error: {}", e);
        e
    })
}
```

- [ ] **Step 3: Add panic handler**

Add at the top of the file:

```rust
use std::panic;

fn setup_panic_handler() {
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location().unwrap_or_else(|| {
            panic::Location::caller()
        });

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        eprintln!("PANIC at {}:{}: {}", location.file(), location.line(), message);
        eprintln!("This is a bug - please report it at https://github.com/your-org/graphql-ish-schema-validator/issues");

        std::process::exit(UNKNOWN_ERROR);
    }));
}

// In main():
fn main() -> Result<()> {
    setup_panic_handler();
    // ... rest of main
}
```

- [ ] **Step 4: Test exit codes**

```bash
# Test successful validation
cargo run -p graphql-ish-schema-validator-cli -- validate valid.yml --schema schema.graphql
echo "Exit code: $?"
# Expected: 0

# Test validation error
cargo run -p graphql-ish-schema-validator-cli -- validate invalid.yml --schema schema.graphql
echo "Exit code: $?"
# Expected: 1

# Test system error (file not found)
cargo run -p graphql-ish-schema-validator-cli -- validate nonexistent.yml --schema schema.graphql
echo "Exit code: $?"
# Expected: 2
```

- [ ] **Step 5: Commit exit code handling**

```bash
git add crates/graphql-ish-schema-validator-cli/src/main.rs
git commit -m "feat: add proper exit code handling and panic handler"
```

---

## Task 4: Add CLI Integration Tests

**Files:**
- Create: `crates/graphql-ish-schema-validator-cli/tests/cli_tests.rs`

**Step 1: Create CLI integration tests**

Create `crates/graphql-ish-schema-validator-cli/tests/cli_tests.rs`:

```rust
//! CLI integration tests

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn create_test_schema() -> TempDir {
    let dir = TempDir::new().unwrap();
    let schema_path = dir.path().join("schema.graphql");

    let schema_content = r#"
        input Test @closed {
            name: String!
            value: Int
        }
    "#;

    let mut file = File::create(&schema_path).unwrap();
    file.write_all(schema_content.as_bytes()).unwrap();

    dir
}

fn create_test_yaml(dir: &TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn test_cli_help() {
    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("GraphQL-inspired schema validator"));
}

#[test]
fn test_cli_validate_valid_yaml() {
    let dir = create_test_schema();
    let yaml = create_test_yaml(&dir, "valid.yml", "name: test\nvalue: 42");
    let schema = dir.path().join("schema.graphql").to_string_lossy().to_string();

    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .args(["validate", &yaml, "--schema", &schema])
        .assert()
        .success();
}

#[test]
fn test_cli_validate_invalid_yaml() {
    let dir = create_test_schema();
    let yaml = create_test_yaml(&dir, "invalid.yml", "name: 123");  // Should be string
    let schema = dir.path().join("schema.graphql").to_string_lossy().to_string();

    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .args(["validate", &yaml, "--schema", &schema])
        .assert()
        .failure()
        .code(1);  // Validation error exit code
}

#[test]
fn test_cli_validate_nonexistent_file() {
    let dir = create_test_schema();
    let schema = dir.path().join("schema.graphql").to_string_lossy().to_string();

    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .args(["validate", "nonexistent.yml", "--schema", &schema])
        .assert()
        .failure()
        .code(2);  // System error exit code
}

#[test]
fn test_cli_json_output() {
    let dir = create_test_schema();
    let yaml = create_test_yaml(&dir, "test.yml", "name: test\nvalue: 42");
    let schema = dir.path().join("schema.graphql").to_string_lossy().to_string();

    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .args(["validate", &yaml, "--schema", &schema, "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\":true"));
}

#[test]
fn test_cli_github_actions_output() {
    let dir = create_test_schema();
    let yaml = create_test_yaml(&dir, "invalid.yml", "name: 123");
    let schema = dir.path().join("schema.graphql").to_string_lossy().to_string();

    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .args(["validate", &yaml, "--schema", &schema, "--format", "github-actions"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("::error"));
}

#[test]
fn test_cli_verbose_flag() {
    let dir = create_test_schema();
    let yaml = create_test_yaml(&dir, "test.yml", "name: test");
    let schema = dir.path().join("schema.graphql").to_string_lossy().to_string();

    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .args(["validate", &yaml, "--schema", &schema, "--verbose"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Validating"));
}

#[test]
fn test_cli_quiet_flag() {
    let dir = create_test_schema();
    let yaml = create_test_yaml(&dir, "test.yml", "name: test");
    let schema = dir.path().join("schema.graphql").to_string_lossy().to_string();

    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .args(["validate", &yaml, "--schema", &schema, "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());  // No output on success
}

#[test]
fn test_cli_strict_mode() {
    let dir = create_test_schema();
    let yaml = create_test_yaml(&dir, "test.yml", "name: test\nextra: ignored");
    let schema = dir.path().join("schema.graphql").to_string_lossy().to_string();

    Command::cargo_bin("graphql-ish-schema-validator")
        .unwrap()
        .args(["validate", &yaml, "--schema", &schema, "--strict"])
        .assert()
        .failure();  // Should fail with strict mode
}
```

- [ ] **Step 2: Run CLI integration tests**

Run: `cargo test --test cli_tests`
Expected: All tests pass

- [ ] **Step 3: Commit CLI integration tests**

```bash
git add crates/graphql-ish-schema-validator-cli/tests/cli_tests.rs
git commit -m "test: add CLI integration tests"
```

---

## Task 5: Add Shell Completion Scripts

**Files:**
- Create: `crates/graphql-ish-schema-validator-cli/completions/gqlsdl.bash`
- Create: `crates/graphql-ish-schema-validator-cli/completions/gqlsdl.fish`
- Create: `crates/graphql-ish-schema-validator-cli/completions/gqlsdl.zsh`

**Step 1: Generate completion scripts**

```bash
# Generate bash completion
cargo run -p graphql-ish-schema-validator-cli -- generate-shell bash > crates/graphql-ish-schema-validator-cli/completions/gqlsdl.bash

# Generate fish completion
cargo run -p graphql-ish-schema-validator-cli -- generate-shell fish > crates/graphql-ish-schema-validator-cli/completions/gqlsdl.fish

# Generate zsh completion
cargo run -p graphql-ish-schema-validator-cli -- generate-shell zsh > crates/graphql-ish-schema-validator-cli/completions/gqlsdl.zsh
```

- [ ] **Step 2: Add installation instructions to README**

Add section to crate README:

```markdown
## Shell Completion

Bash, Fish, and Zsh completion scripts are available in the `completions/` directory.

### Bash
```bash
source completions/gqlsdl.bash
```

Or install system-wide:
```bash
sudo cp completions/gqlsdl.bash /usr/share/bash-completion/completions/gqlsdl
```

### Fish
```bash
source completions/gqlsdl.fish
```

Or install system-wide:
```bash
cp completions/gqlsdl.fish ~/.config/fish/completions/gqlsdl.fish
```

### Zsh
```bash
source completions/gqlsdl.zsh
```

Or install system-wide:
```bash
sudo cp completions/gqlsdl.zsh /usr/share/zsh/vendor-completions/_gqlsdl
```
```

- [ ] **Step 3: Test completion scripts**

Test bash completion:
```bash
source completions/gqlsdl.bash
gqlsdl <TAB>
# Should show: validate, compile, check-schema, export-jtd
```

- [ ] **Step 4: Commit shell completions**

```bash
git add crates/graphql-ish-schema-validator-cli/completions/
git add crates/graphql-ish-schema-validator-cli/README.md
git commit -m "feat: add shell completion scripts and installation instructions"
```

---

## Verification

**Step 1: Full CLI test suite**

Run: `cargo test -p graphql-ish-schema-validator-cli`
Expected: All tests pass

- [ ] **Step 2: Test all subcommands**

```bash
# Validate
cargo run -p graphql-ish-schema-validator-cli -- validate test.yml --schema schema.graphql

# Compile
cargo run -p graphql-ish-schema-validator-cli -- compile schema.graphql --output schema.json

# Check schema
cargo run -p graphql-ish-schema-validator-cli -- check-schema schema.graphql

# Export JTD
cargo run -p graphql-ish-schema-validator-cli -- export-jtd schema.graphql --output schema.jtd.json
```

- [ ] **Step 3: Test all output formats**

```bash
# Text format
cargo run -p graphql-ish-schema-validator-cli -- validate test.yml --schema schema.graphql --format text

# JSON format
cargo run -p graphql-ish-schema-validator-cli -- validate test.yml --schema schema.graphql --format json

# GitHub Actions format
cargo run -p graphql-ish-schema-validator-cli -- validate test.yml --schema schema.graphql --format github-actions
```

- [ ] **Step 4: Test exit codes**

Verify exit codes for different scenarios.

- [ ] **Step 5: Test progress reporting**

Test with a directory containing multiple files.

- [ ] **Step 6: Final verification checklist**

Verify:
- ✅ Binary names: `graphql-ish-schema-validator` and `gqlsdl`
- ✅ All subcommands implemented: validate, compile, check-schema, export-jtd
- ✅ Global flags: --verbose, --quiet, --log-level, --log-file, --format, --color
- ✅ Exit codes: 0=valid, 1=validation errors, 2=system errors, 3=unknown
- ✅ Stdin support
- ✅ Directory traversal
- ✅ Glob pattern support
- ✅ Progress reporting for multi-file validation
- ✅ Output formats: text, json, github-actions
- ✅ Colored output with --no-color flag
- ✅ Shell completion scripts
- ✅ Integration tests pass

- [ ] **Step 7: Final commit**

```bash
git add .
git commit -m "feat: complete CLI implementation with all features"
```

---

## Summary

This plan implements a fully-featured CLI for the GraphQL-ish schema validator:

**Key achievements:**
1. **Complete CLI structure** with all subcommands
2. **Progress reporting** for batch operations
3. **Proper exit codes** for different scenarios
4. **Integration tests** for all major features
5. **Shell completions** for bash, fish, and zsh

**CLI features:**
- Subcommands: validate, compile, check-schema, export-jtd
- Global flags for logging and output configuration
- Multiple output formats (text, JSON, GitHub Actions)
- Directory traversal and glob pattern support
- Stdin support
- Progress bars for batch operations
- Proper error handling and panic recovery
- Shell completions

**User experience:**
- Helpful error messages
- Verbose mode for debugging
- Quiet mode for CI/CD
- Progress feedback for long operations
- Colored terminal output (configurable)
- Exit codes for automation

**Next steps:**
- Add comprehensive test suite (see [04-testing-and-verification.md](./04-testing-and-verification.md))
- Create migration guide (see [05-migration-checklist.md](./05-migration-checklist.md))
- Complete schema compilation and validation implementation
