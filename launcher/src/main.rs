use clap::Parser;
use env_logger;
use log::{info, LevelFilter};

// Include the version generated at build time
include!(concat!(env!("OUT_DIR"), "/version.rs"));

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: Option<String>,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,

    /// Set the logging level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

fn parse_log_level(level: &str) -> LevelFilter {
    match level.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => {
            eprintln!("Warning: Invalid log level '{}', using 'info'", level);
            LevelFilter::Info
        }
    }
}

fn main() {
    // Parse command line arguments first
    let args = Args::parse();

    // Initialize logging with the specified level
    env_logger::Builder::from_default_env()
        .filter_level(parse_log_level(&args.log_level))
        .init();

    // Log the application version
    info!("Starting {} version {}", env!("CARGO_PKG_NAME"), VERSION);

    // Log the parsed arguments
    info!("args: {:?}", args);

    // Example usage of the parsed arguments
    let name = args.name.as_deref().unwrap_or("world");
    
    for _ in 0..args.count {
        println!("Hello, {}!", name);
    }
}
