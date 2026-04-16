use clap::{Parser, Subcommand};
use graphql_ish_schema_validator::{
    validate_json_from_schema, validate_yaml_from_schema, LogLevel, ValidationMode,
    ValidationOptions,
};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "graphql-ish-schema-validator",
    version,
    about = "Validate YAML/JSON against GraphQL SDL schemas"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a document against a schema
    Validate {
        /// Path to GraphQL SDL schema file
        #[arg(short, long)]
        schema: PathBuf,

        /// Path to YAML/JSON document (use - for stdin)
        #[arg(short, long)]
        input: Option<String>,

        /// Use strict validation (reject unknown fields, default)
        #[arg(long, default_value_t = true)]
        strict: bool,

        /// Use open validation (allow unknown fields)
        #[arg(long)]
        open: bool,

        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Root schema name (auto-detected if omitted)
        #[arg(long)]
        root: Option<String>,

        /// Log level: silent, error, warn, info, debug, trace
        #[arg(long, default_value = "warn")]
        log_level: String,
    },

    /// Compile a schema to IR (for debugging)
    Compile {
        /// Path to GraphQL SDL schema file
        #[arg(short, long)]
        schema: PathBuf,
    },
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate {
            schema,
            input,
            strict,
            open,
            format,
            root,
            log_level,
        } => {
            run_validate(
                &schema,
                input.as_deref(),
                strict,
                open,
                &format,
                root.as_deref(),
                &log_level,
            )?;
        }
        Commands::Compile { schema } => {
            run_compile(&schema)?;
        }
    }

    Ok(())
}

fn run_validate(
    schema_path: &PathBuf,
    input_path: Option<&str>,
    _strict: bool,
    open: bool,
    format: &str,
    root: Option<&str>,
    log_level: &str,
) -> miette::Result<()> {
    // Initialize tracing subscriber based on log level
    let level = match log_level {
        "silent" => "off",
        "error" => "error",
        "warn" => "warn",
        "info" => "info",
        "debug" => "debug",
        "trace" => "trace",
        _ => return Err(miette::miette!("Invalid log level: {log_level}")),
    };
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(EnvFilter::new(level))
        .init();

    // Read schema file
    let schema_content = std::fs::read_to_string(schema_path)
        .map_err(|e| miette::miette!("failed to read schema: {e}"))?;

    // Read input document
    let doc_content = match input_path {
        Some("-") | None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| miette::miette!("stdin: {e}"))?;
            buf
        }
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| miette::miette!("failed to read input: {e}"))?,
    };

    // Build validation options
    let mode = if open {
        ValidationMode::Open
    } else {
        ValidationMode::Strict
    };

    let log_level_enum = match log_level {
        "silent" => LogLevel::Silent,
        "error" => LogLevel::Error,
        "warn" => LogLevel::Warn,
        "info" => LogLevel::Info,
        "debug" => LogLevel::Debug,
        "trace" => LogLevel::Trace,
        _ => LogLevel::Warn,
    };

    let options = ValidationOptions {
        mode,
        root_schema: root.map(String::from),
        max_depth: 64,
        log_level: log_level_enum,
    };

    // Detect YAML vs JSON and validate
    let is_json =
        doc_content.trim_start().starts_with('{') || doc_content.trim_start().starts_with('[');

    let result = if is_json {
        validate_json_from_schema(&doc_content, &schema_content, &options)
            .map_err(|e| miette::miette!("Validation error: {e}"))?
    } else {
        validate_yaml_from_schema(&doc_content, &schema_content, &options)
            .map_err(|e| miette::miette!("Validation error: {e}"))?
    };

    // Output results
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&result)
                .map_err(|e| miette::miette!("JSON serialization error: {e}"))?;
            println!("{json}");
        }
        _ => {
            if result.valid {
                println!("✓ Valid");
            } else {
                for err in &result.errors {
                    eprintln!(
                        "error[{}]: {} (at {})",
                        err.code, err.message, err.instance_path
                    );
                    if let Some(hint) = &err.hint {
                        eprintln!("  hint: {hint}");
                    }
                }
            }
        }
    }

    if !result.valid {
        std::process::exit(1);
    }

    Ok(())
}

fn run_compile(schema_path: &PathBuf) -> miette::Result<()> {
    let sdl_content = std::fs::read_to_string(schema_path)
        .map_err(|e| miette::miette!("failed to read schema: {e}"))?;

    let ast = graphql_ish_schema_validator_parser::extract_ast(&sdl_content)
        .map_err(|errs| miette::miette!("SDL parse errors: {:?}", errs))?;

    let bundle = graphql_ish_schema_validator_compiler::compile(&ast)
        .map_err(|errs| miette::miette!("compile errors: {:?}", errs))?;

    let json = serde_json::to_string_pretty(&bundle)
        .map_err(|e| miette::miette!("JSON serialization: {e}"))?;
    println!("{json}");

    Ok(())
}
