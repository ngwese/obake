// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! `obake` CLI — GraphQL client for obake-session.

use clap::{Parser, Subcommand};
use env_logger::Env;
use obake::format;
use obake::{default_config_path, load_config, resolve_endpoint, Client, Kind};
use serde_json::Value;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "obake", about = "Client for the obake-session GraphQL API")]
struct Cli {
    /// Client config path.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Named host from the config file.
    #[arg(long)]
    host: Option<String>,
    /// Override endpoint URL.
    #[arg(long)]
    endpoint: Option<String>,
    /// Override bearer token.
    #[arg(long)]
    token: Option<String>,
    /// Print GraphQL JSON instead of a human-readable table.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List rigs.
    Rigs,
    /// Show one rig, or install/remove.
    Rig {
        #[command(subcommand)]
        action: Option<RigAction>,
        /// Rig name when querying.
        name: Option<String>,
    },
    /// List shapes.
    Shapes,
    /// List recorded sessions.
    Sessions {
        #[arg(long)]
        rig: Option<String>,
    },
    /// Show the active session.
    Session,
    /// Start a session.
    Start {
        rig: String,
        #[arg(long, default_value = "FRESH")]
        mode: String,
        #[arg(long)]
        fork_from: Option<String>,
    },
    /// Stop the active session.
    Stop,
    /// Replace system links (JSON array on stdin or --links).
    Links {
        #[arg(long)]
        links: Option<String>,
    },
    /// List session work-area files.
    Files { id: String },
    /// Fetch one session file (prints JSON with base64).
    File { id: String, path: String },
    /// Delete a recorded session.
    #[command(name = "delete-session")]
    DeleteSession { id: String },
    /// Live PipeWire graph.
    Graph,
    /// Ravenna stream replacement.
    Ravenna {
        #[command(subcommand)]
        cmd: RavennaCommands,
    },
    /// Watch session/link subscriptions.
    Watch {
        #[arg(long, default_value = "session")]
        what: String,
    },
}

#[derive(Subcommand, Debug)]
enum RigAction {
    /// Install a rig document from a file.
    Install {
        name: String,
        #[arg(long)]
        file: PathBuf,
    },
    /// Remove a rig by name.
    Remove { name: String },
}

#[derive(Subcommand, Debug)]
enum RavennaCommands {
    /// Replace sources and sinks (JSON on stdin: {sources,sinks}).
    Set,
}

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_path = cli.config.unwrap_or_else(default_config_path);
    let config = load_config(&config_path)?;
    let resolved = resolve_endpoint(
        &config,
        cli.host.as_deref(),
        cli.endpoint.as_deref(),
        cli.token.as_deref(),
        None,
    )?;
    let client = Client::from_resolved(&resolved);
    let as_json = cli.json;

    let (kind, data) = match cli.command {
        Commands::Rigs => (Kind::Rigs, client.rigs().await?),
        Commands::Rig { action, name } => match action {
            Some(RigAction::Install { name, file }) => {
                let doc = std::fs::read_to_string(file)?;
                (Kind::InstallRig, client.install_rig(&name, &doc).await?)
            }
            Some(RigAction::Remove { name }) => {
                (Kind::RemoveRig, client.remove_rig(&name).await?)
            }
            None => {
                let name = name.ok_or("rig name required")?;
                (Kind::Rig, client.rig(&name).await?)
            }
        },
        Commands::Shapes => (Kind::Shapes, client.shapes().await?),
        Commands::Sessions { rig } => (Kind::Sessions, client.sessions(rig.as_deref()).await?),
        Commands::Session => (Kind::Session, client.session().await?),
        Commands::Start {
            rig,
            mode,
            fork_from,
        } => (
            Kind::Start,
            client.start(&rig, &mode, fork_from.as_deref()).await?,
        ),
        Commands::Stop => (Kind::Stop, client.stop().await?),
        Commands::Links { links } => {
            let text = match links {
                Some(s) => s,
                None => {
                    let mut buf = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                    buf
                }
            };
            let value: Value = serde_json::from_str(&text)?;
            (Kind::Links, client.set_links(value).await?)
        }
        Commands::Files { id } => (Kind::Files, client.files(&id).await?),
        Commands::File { id, path } => (Kind::File, client.file(&id, &path).await?),
        Commands::DeleteSession { id } => (Kind::DeleteSession, client.delete_session(&id).await?),
        Commands::Graph => (Kind::Graph, client.graph().await?),
        Commands::Ravenna { cmd } => match cmd {
            RavennaCommands::Set => {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                let value: Value = serde_json::from_str(&buf)?;
                let sources = value
                    .get("sources")
                    .cloned()
                    .unwrap_or(Value::Array(vec![]));
                let sinks = value.get("sinks").cloned().unwrap_or(Value::Array(vec![]));
                (Kind::Ravenna, client.ravenna_set(sources, sinks).await?)
            }
        },
        Commands::Watch { .. } => (Kind::Watch, Value::Null),
    };

    if as_json {
        if !data.is_null() {
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
    } else {
        print!("{}", format::render(kind, &data));
    }
    Ok(())
}
