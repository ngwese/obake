use clap::Parser;
use env_logger;
use log::{LevelFilter, info, trace};

use systemd_zbus::{ManagerProxy, Mode, zbus::Connection};

mod config;

// Include the version generated at build time
include!(concat!(env!("OUT_DIR"), "/version.rs"));

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Set the logging level
    #[arg(short, long, default_value = "info", global = true)]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// List available shapes
    ShapeList,

    /// Start a shape
    ShapeStart {
        /// Name of the shape to start
        name: String,
    },

    /// Stop a shape
    ShapeStop {
        /// Name of the shape to stop
        name: String,
    },
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Handle subcommands
    match &args.command {
        Commands::ShapeList => {
            shape_list().await?;
        }
        Commands::ShapeStart { name } => {
            shape_start(&name).await?;
        }
        Commands::ShapeStop { name } => {
            shape_stop(&name).await?;
        }
    }

    Ok(())
}

async fn get_manager() -> Result<ManagerProxy<'static>, Box<dyn std::error::Error>> {
    let connection = Connection::session().await?;
    // let connection = Connection::system().await?;
    trace!("connection: {:?}", connection);
    let manager = ManagerProxy::new(&connection).await?;
    trace!("manager: {:?}", manager);
    Ok(manager)
}

async fn shape_list() -> Result<(), Box<dyn std::error::Error>> {
    let manager = get_manager().await?;

    for path in manager.unit_path().await? {
        info!("unit path: {:?}", path);
    }

    // let units = manager.list_units().await?;
    let units = manager.list_unit_files().await?;
    for unit in units {
        info!("unit: {:?}", unit);
    }

    Ok(())
}

async fn shape_start(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let manager = get_manager().await?;
    manager.start_unit(name, Mode::Replace).await?;
    info!("started unit: {:?}", name);
    Ok(())
}

async fn shape_stop(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let manager = get_manager().await?;
    manager.stop_unit(name, Mode::Replace).await?;
    info!("stopped unit: {:?}", name);
    Ok(())
}
