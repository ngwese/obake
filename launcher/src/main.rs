use anyhow::{Context, Result};
use clap::Parser;
use env_logger;
use log::{LevelFilter, debug, info, trace};
use systemd_zbus::{ManagerProxy, Mode, zbus::Connection};

mod config;
mod setup;

// Include the version generated at build time
include!(concat!(env!("OUT_DIR"), "/version.rs"));

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Set the logging level
    #[arg(short, long, default_value = "info", global = true, env = "RUST_LOG")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Manage shapes
    Shape {
        #[command(subcommand)]
        action: ShapeCommands,
    },
    /// Manage units
    Unit {
        #[command(subcommand)]
        action: UnitCommands,
    },
    /// Manage setups
    Setup {
        #[command(subcommand)]
        action: SetupCommands,
    },
    /// Manage audio interfaces
    Interface {
        #[command(subcommand)]
        action: InterfaceCommands,
    },
}

#[derive(Parser, Debug)]
enum ShapeCommands {
    /// List available shapes
    List,
}

#[derive(Parser, Debug)]
enum UnitCommands {
    /// Start a unit
    Start {
        /// Name of the unit to start
        name: String,
    },
    /// Stop a unit
    Stop {
        /// Name of the unit to stop
        name: String,
    },
}

#[derive(Parser, Debug)]
enum SetupCommands {
    /// Start a setup
    Start,
    /// Stop a setup
    Stop,
    /// List available setups
    List,
}

#[derive(Parser, Debug)]
enum InterfaceCommands {
    /// List available audio interfaces
    List,
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
async fn main() -> Result<()> {
    // Parse command line arguments first
    let args = Args::parse();

    // Initialize logging with the specified level
    env_logger::Builder::from_default_env()
        .filter_level(parse_log_level(&args.log_level))
        .init();

    // Log the application version
    info!("Starting {} version {}", env!("CARGO_PKG_NAME"), VERSION);

    // Log the parsed arguments
    debug!("args: {:?}", args);

    // Handle subcommands
    let result = match &args.command {
        Commands::Shape { action } => match action {
            ShapeCommands::List => shape_list().await,
        },
        Commands::Unit { action } => match action {
            UnitCommands::Start { name } => unit_start(&name).await,
            UnitCommands::Stop { name } => unit_stop(&name).await,
        },
        Commands::Setup { action } => match action {
            SetupCommands::Start => setup_start().await,
            SetupCommands::Stop => setup_stop().await,
            SetupCommands::List => setup_list().await,
        },
        Commands::Interface { action } => match action {
            InterfaceCommands::List => audio_interface_list().await,
        },
    };

    if let Err(e) = result {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }

    result
}

async fn get_manager() -> Result<ManagerProxy<'static>> {
    let connection = Connection::session().await?;
    // let connection = Connection::system().await?;
    trace!("connection: {:?}", connection);
    let manager = ManagerProxy::new(&connection).await?;
    trace!("manager: {:?}", manager);
    Ok(manager)
}

async fn shape_list() -> Result<()> {
    let config = config::Config::load()?;
    let images_dir = config.get_images_dir();
    debug!("images directory: {}", images_dir);
    let images = std::fs::read_dir(images_dir)
        .with_context(|| format!("Failed to read images directory: {}", images_dir))?;

    let mut found_image = false;
    for image in images {
        info!("image: {:?}", image);
        found_image = true;
    }

    if !found_image {
        return Err(anyhow::anyhow!(
            "No images found in directory: {}",
            images_dir
        ));
    }

    Ok(())
}

async fn unit_start(name: &str) -> Result<()> {
    let manager = get_manager().await?;
    manager.start_unit(name, Mode::Replace).await?;
    info!("started unit: {:?}", name);
    Ok(())
}

async fn unit_stop(name: &str) -> Result<()> {
    let manager = get_manager().await?;
    manager.stop_unit(name, Mode::Replace).await?;
    info!("stopped unit: {:?}", name);
    Ok(())
}

async fn audio_interface_list() -> Result<()> {
    let config = config::Config::load()?;
    for interface in config.list_audio_interfaces() {
        info!("audio interface: {:?}", interface);
    }
    Ok(())
}

async fn setup_start() -> Result<()> {
    let config = config::Config::load()?;
    let setups_dir = config.get_setups_dir();
    info!("Setups directory: {}", setups_dir);
    let setup_files = std::fs::read_dir(setups_dir)?;
    for setup_file in setup_files {
        info!("Setup file: {:?}", setup_file);
    }
    info!("Starting setup");

    // Load setup configuration
    let setup_config = setup::SetupConfig::load_from_path("setup.toml")?;
    info!("Loaded setup configuration: {:?}", setup_config);

    // Get the default interface from setup
    let interface_name = &setup_config.setup.interface;
    info!("Using interface: {}", interface_name);

    // Load main configuration to get interface details
    let config = config::Config::load()?;
    if let Some(interface) = config.get_audio_interface(interface_name) {
        info!("Interface type: {}", interface.interface_type);

        // Start the interface unit if it has one
        if let Some(unit) = &interface.unit {
            info!("Starting interface unit: {}", unit);
            let manager = get_manager().await?;
            manager.start_unit(unit, Mode::Replace).await?;
            info!("Started interface unit: {}", unit);
        } else {
            info!("Interface has no systemd unit to start");
        }
    } else {
        return Err(anyhow::anyhow!(
            "Interface '{}' not found in configuration",
            interface_name
        ));
    }

    // Start shapes from setup
    for shape_name in &setup_config.setup.shapes {
        info!("Starting shape: {}", shape_name);

        if let Some(shape) = setup_config.get_shape(shape_name) {
            if let Some(image) = &shape.image {
                info!("Shape image: {}", image);
                // TODO: Implement shape container startup logic
            } else {
                info!("Shape has no container image");
            }

            if let Some(env) = &shape.env {
                info!("Shape environment variables: {:?}", env);
            }
        } else {
            info!("Shape '{}' not found in configuration", shape_name);
        }
    }

    info!("Setup started successfully");
    Ok(())
}

async fn setup_stop() -> Result<()> {
    info!("Stopping setup");

    // Load setup configuration
    let setup_config = setup::SetupConfig::load_from_path("setup.toml")?;
    info!("Loaded setup configuration: {:?}", setup_config);

    // Stop shapes from setup (in reverse order)
    for shape_name in setup_config.setup.shapes.iter().rev() {
        info!("Stopping shape: {}", shape_name);

        if let Some(shape) = setup_config.get_shape(shape_name) {
            if let Some(image) = &shape.image {
                info!("Stopping shape container: {}", image);
                // TODO: Implement shape container stop logic
            } else {
                info!("Shape has no container to stop");
            }
        } else {
            info!("Shape '{}' not found in configuration", shape_name);
        }
    }

    // Stop the interface unit
    let interface_name = &setup_config.setup.interface;
    info!("Stopping interface: {}", interface_name);

    let config = config::Config::load()?;
    if let Some(interface) = config.get_audio_interface(interface_name) {
        if let Some(unit) = &interface.unit {
            info!("Stopping interface unit: {}", unit);
            let manager = get_manager().await?;
            manager.stop_unit(unit, Mode::Replace).await?;
            info!("Stopped interface unit: {}", unit);
        } else {
            info!("Interface has no systemd unit to stop");
        }
    } else {
        info!("Interface '{}' not found in configuration", interface_name);
    }

    info!("Setup stopped successfully");
    Ok(())
}

async fn setup_list() -> Result<()> {
    let config = config::Config::load()?;
    let setups_dir = config.get_setups_dir();
    debug!("setups directory: {}", setups_dir);

    let setup_files = std::fs::read_dir(setups_dir)
        .with_context(|| format!("Failed to read setups directory: {}", setups_dir))?;

    let mut found_setup = false;
    for setup_file in setup_files {
        info!("Setup file: {:?}", setup_file);
        found_setup = true;
    }

    if !found_setup {
        return Err(anyhow::anyhow!(
            "No setups found in directory: {}",
            setups_dir
        ));
    }

    Ok(())
}
