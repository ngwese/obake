// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! `obake-session` binary: `run` serves GraphQL.

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::info;
use obake_session::launch::SingularityLauncher;
use obake_session::links::PwLinkGraph;
use obake_session::paths::Paths;
use obake_session::service::Service;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "obake-session", about = "Rig and session daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Serve GraphQL on the Unix socket and TCP listener.
    Run {
        /// Override config directory.
        #[arg(long)]
        config_dir: Option<PathBuf>,
        /// Override state directory.
        #[arg(long)]
        state_dir: Option<PathBuf>,
        /// Override share directory.
        #[arg(long)]
        share_dir: Option<PathBuf>,
        /// Override runtime directory.
        #[arg(long)]
        runtime_dir: Option<PathBuf>,
        /// Override default image directory.
        #[arg(long)]
        image_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Run {
            config_dir,
            state_dir,
            share_dir,
            runtime_dir,
            image_dir,
        } => {
            let mut paths = Paths::from_env();
            if let Some(p) = config_dir {
                paths.config_dir = p;
            }
            if let Some(p) = state_dir {
                paths.state_dir = p;
            }
            if let Some(p) = share_dir {
                paths.share_dir = p;
            }
            if let Some(p) = runtime_dir {
                paths.runtime_dir = p;
            }
            if let Some(p) = image_dir {
                paths.default_image_dir = p;
            }
            let launcher = Arc::new(SingularityLauncher::new());
            let graph = Arc::new(PwLinkGraph::new());
            let service = Service::new(paths, launcher, graph)?;
            service.restore()?;
            info!(
                "obake-session listening on {} and {}",
                service.paths().api_socket().display(),
                service.host().listen
            );
            obake_session::graphql::serve(service).await?;
        }
    }
    Ok(())
}
