// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! `obake` CLI — GraphQL / HTTP client for obake-session.

use clap::{Parser, Subcommand};
use env_logger::Env;
use obake::format;
use obake::xfer::{
    expand_local_sources, is_dir_dest, is_remote_dir_dest, local_target_for_get,
    match_remote_sources, remote_target_for_put, split_src_dest,
};
use obake::{
    default_config_path, load_config, missing_remote_setup, resolve_endpoint, Client, Kind,
    Resolved,
};
use serde_json::Value;
use std::io;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "obake", about = "Client for the obake-session API")]
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
    /// Rig documents.
    Rig {
        #[command(subcommand)]
        action: RigCmd,
    },
    /// Shape profiles.
    Shape {
        #[command(subcommand)]
        action: ShapeCmd,
    },
    /// Sessions.
    Session {
        #[command(subcommand)]
        action: SessionCmd,
    },
    /// System links.
    Link {
        #[command(subcommand)]
        action: LinkCmd,
    },
    /// Session work-area files.
    File {
        #[command(subcommand)]
        action: FileCmd,
    },
    /// Live PipeWire graph.
    Graph {
        #[command(subcommand)]
        action: GraphCmd,
    },
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
    /// Enroll this CLI, approve a browser, or rotate the host bearer.
    Auth {
        #[command(subcommand)]
        action: AuthCmd,
    },
    /// Host OS update over SSH (no session API).
    Device {
        #[command(subcommand)]
        action: DeviceCmd,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCmd {
    /// Copy the host bearer and certificate fingerprint over SSH.
    Login {
        /// `ssh-user@host`, for example `obake@studio.local`.
        target: String,
    },
    /// Approve a browser device code.
    Approve {
        /// Code shown by the browser.
        code: String,
    },
    /// Replace the host bearer. Browser cookies and other copies stop working.
    Rotate,
    /// Print bearer, port, and certificate fingerprint from the local socket.
    Credential,
}

#[derive(Subcommand, Debug)]
enum DeviceCmd {
    /// Push a RAUC bundle over SSH and apply it with cleanup + reboot.
    Update {
        /// `ssh-user@host`, for example `obake@studio.local`.
        #[arg(long)]
        target: String,
        /// Local `.raucb` file or directory of `obake-<semver>-<slot>.raucb`.
        path: PathBuf,
        /// Skip slot and semver checks (file path only).
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum RigCmd {
    /// List installed rigs.
    Ls,
    /// Show one rig.
    Describe { name: String },
    /// Scaffold a local directory package (`<name>/rig.yaml` + `work/`).
    Init {
        /// Package directory name.
        name: String,
        /// Parent directory (default: current directory).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Install a rig from a local YAML file or directory package.
    Create {
        /// Rig name on the host.
        name: String,
        /// Local `.yaml` file or package directory containing `rig.yaml`.
        #[arg(long)]
        path: PathBuf,
    },
    /// Remove a rig by name.
    Delete { name: String },
}

#[derive(Subcommand, Debug)]
enum ShapeCmd {
    /// List shapes.
    Ls,
}

#[derive(Subcommand, Debug)]
enum SessionCmd {
    /// List recorded sessions.
    Ls {
        #[arg(long)]
        rig: Option<String>,
    },
    /// Describe the active session, or one recorded session by id.
    Describe { id: Option<String> },
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
    /// Delete an inactive recorded session.
    Delete { id: String },
}

#[derive(Subcommand, Debug)]
enum LinkCmd {
    /// Replace system links (JSON array on stdin or --links).
    Set {
        #[arg(long)]
        links: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum FileCmd {
    /// List work-area files.
    Ls {
        #[arg(long)]
        session: Option<String>,
    },
    /// Download work-area files (globs / directories; last arg is destination).
    Get {
        /// Remote paths, globs, or directories, then local destination
        /// (`.`, `-`, file, or directory).
        #[arg(required = true, num_args = 2..)]
        args: Vec<String>,
        #[arg(long)]
        session: Option<String>,
    },
    /// Upload local files (globs / directories; last arg is destination).
    Put {
        /// Local paths or globs, then remote destination (`.`, file, or
        /// directory).
        #[arg(required = true, num_args = 2..)]
        args: Vec<String>,
        #[arg(long)]
        session: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum GraphCmd {
    /// List live PipeWire nodes.
    Ls,
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
    match &cli.command {
        Commands::Rig {
            action: RigCmd::Init { name, path },
        } => {
            let parent = path
                .clone()
                .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
            let root = obake::rig::init_package(&parent, name)?;
            println!("{}", root.display());
            return Ok(());
        }
        Commands::Auth {
            action: AuthCmd::Login { target },
        } => {
            return obake::auth::login(&config_path, target).map_err(|e| e.into());
        }
        Commands::Auth {
            action: AuthCmd::Credential,
        } => {
            return obake::auth::print_credential().await.map_err(|e| e.into());
        }
        Commands::Auth {
            action: AuthCmd::Approve { code },
        } => {
            let config = load_config(&config_path)?;
            let resolved = resolve_endpoint(
                &config,
                cli.host.as_deref(),
                cli.endpoint.as_deref(),
                cli.token.as_deref(),
                None,
            )?;
            ensure_setup(&config_path, &resolved)?;
            return obake::auth::approve(&resolved, code)
                .await
                .map_err(|e| e.into());
        }
        Commands::Auth {
            action: AuthCmd::Rotate,
        } => {
            let config = load_config(&config_path)?;
            let resolved = resolve_endpoint(
                &config,
                cli.host.as_deref(),
                cli.endpoint.as_deref(),
                cli.token.as_deref(),
                None,
            )?;
            ensure_setup(&config_path, &resolved)?;
            return obake::auth::rotate(&config_path, &resolved)
                .await
                .map_err(|e| e.into());
        }
        Commands::Device {
            action: DeviceCmd::Update {
                target,
                path,
                force,
            },
        } => {
            return obake::device::update(target, path, *force).map_err(|e| e.into());
        }
        _ => {}
    }
    let config = load_config(&config_path)?;
    let resolved = resolve_endpoint(
        &config,
        cli.host.as_deref(),
        cli.endpoint.as_deref(),
        cli.token.as_deref(),
        None,
    )?;
    ensure_setup(&config_path, &resolved)?;
    let client = Client::from_resolved(&resolved);
    let as_json = cli.json;

    match cli.command {
        Commands::File {
            action: FileCmd::Get { args, session },
        } => {
            let (sources, dest) = split_src_dest(&args)?;
            let id = resolve_session_id(&client, session.as_deref()).await?;
            let entries = session_file_entries(&client, &id).await?;
            let remotes = match_remote_sources(&entries, &sources)?;
            if remotes.is_empty() {
                return Err("no remote files matched".into());
            }
            if dest == "-" && remotes.len() != 1 {
                return Err("stdout (-) requires exactly one remote file".into());
            }
            let multi = remotes.len() > 1 || is_dir_dest(&dest);
            let mut results = Vec::new();
            for remote in &remotes {
                let local_target = local_target_for_get(remote, &dest, multi)?;
                let written = stream_get_to(&client, &id, remote, &local_target).await?;
                if !as_json && dest != "-" {
                    println!("wrote {written} bytes to {local_target}");
                }
                results.push(serde_json::json!({
                    "bytes": written,
                    "path": remote,
                    "dest": local_target,
                }));
            }
            if as_json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            return Ok(());
        }
        Commands::File {
            action: FileCmd::Put { args, session },
        } => {
            let (sources, dest) = split_src_dest(&args)?;
            let id = resolve_session_id(&client, session.as_deref()).await?;
            let locals = expand_local_sources(&sources)?;
            if locals.is_empty() {
                return Err("no local files matched".into());
            }
            let entries = session_file_entries(&client, &id).await?;
            let multi =
                locals.len() > 1 || is_dir_dest(&dest) || is_remote_dir_dest(&dest, &entries);
            let mut results = Vec::new();
            for local in &locals {
                let remote = remote_target_for_put(local, &dest, multi);
                let written = client
                    .put_file_from_path(&id, &remote, &local.path)
                    .await?;
                if !as_json {
                    println!("wrote {written} bytes to {remote}");
                }
                results.push(serde_json::json!({
                    "bytes": written,
                    "path": remote,
                    "local": local.path.display().to_string(),
                }));
            }
            if as_json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
            return Ok(());
        }
        _ => {}
    }

    let (kind, data) = match cli.command {
        Commands::Rig { action } => match action {
            RigCmd::Ls => (Kind::Rigs, client.rigs().await?),
            RigCmd::Describe { name } => (Kind::Rig, client.rig(&name).await?),
            RigCmd::Create { name, path } => {
                if path.is_dir() {
                    let tar = obake::rig::pack_directory(&path)?;
                    client.install_rig_package(&name, &tar).await?;
                    (Kind::InstallRig, serde_json::json!({ "installRig": { "name": name } }))
                } else {
                    let doc = std::fs::read_to_string(&path)?;
                    (Kind::InstallRig, client.install_rig(&name, &doc).await?)
                }
            }
            RigCmd::Init { .. } => unreachable!("handled before client"),
            RigCmd::Delete { name } => (Kind::RemoveRig, client.remove_rig(&name).await?),
        },
        Commands::Shape { action } => match action {
            ShapeCmd::Ls => (Kind::Shapes, client.shapes().await?),
        },
        Commands::Session { action } => match action {
            SessionCmd::Ls { rig } => (Kind::Sessions, client.sessions(rig.as_deref()).await?),
            SessionCmd::Describe { id: None } => {
                let data = client.session().await?;
                if data.get("session").map(|s| s.is_null()).unwrap_or(true) {
                    return Err("no active session".into());
                }
                (Kind::Session, data)
            }
            SessionCmd::Describe { id: Some(id) } => {
                let active = client.session().await?;
                let active_id = active
                    .get("session")
                    .and_then(|s| s.get("id"))
                    .and_then(|v| v.as_str());
                if active_id == Some(id.as_str()) {
                    (Kind::Session, active)
                } else {
                    let listed = client.sessions(None).await?;
                    let found = listed
                        .get("sessions")
                        .and_then(|a| a.as_array())
                        .and_then(|arr| {
                            arr.iter()
                                .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                                .cloned()
                        })
                        .ok_or_else(|| format!("session not found: {id}"))?;
                    (Kind::Session, serde_json::json!({ "session": found }))
                }
            }
            SessionCmd::Start {
                rig,
                mode,
                fork_from,
            } => (
                Kind::Start,
                client.start(&rig, &mode, fork_from.as_deref()).await?,
            ),
            SessionCmd::Stop => (Kind::Stop, client.stop().await?),
            SessionCmd::Delete { id } => (Kind::DeleteSession, client.delete_session(&id).await?),
        },
        Commands::Link { action } => match action {
            LinkCmd::Set { links } => {
                let text = match links {
                    Some(s) => s,
                    None => {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut io::stdin(), &mut buf)?;
                        buf
                    }
                };
                let value: Value = serde_json::from_str(&text)?;
                (Kind::Links, client.set_links(value).await?)
            }
        },
        Commands::File {
            action: FileCmd::Ls { session },
        } => {
            let id = resolve_session_id(&client, session.as_deref()).await?;
            (Kind::Files, client.files(&id).await?)
        }
        Commands::File {
            action: FileCmd::Get { .. } | FileCmd::Put { .. },
        } => unreachable!(),
        Commands::Graph { action } => match action {
            GraphCmd::Ls => (Kind::Graph, client.graph().await?),
        },
        Commands::Ravenna { cmd } => match cmd {
            RavennaCommands::Set => {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut io::stdin(), &mut buf)?;
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
        Commands::Auth { .. } => unreachable!("auth commands return before dispatch"),
        Commands::Device { .. } => unreachable!("device commands return before dispatch"),
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

fn ensure_setup(
    config_path: &Path,
    resolved: &Resolved,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(hint) = missing_remote_setup(config_path, resolved) {
        return Err(hint.into());
    }
    Ok(())
}

async fn resolve_session_id(
    client: &Client,
    session: Option<&str>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(id) = session {
        return Ok(id.to_string());
    }
    let data = client.session().await?;
    data.get("session")
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "no active session".into())
}

async fn session_file_entries(
    client: &Client,
    session_id: &str,
) -> Result<Vec<(String, bool)>, Box<dyn std::error::Error + Send + Sync>> {
    let listing = client.files(session_id).await?;
    let files = listing
        .get("sessionFiles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(files
        .iter()
        .filter_map(|f| {
            let path = f.get("path")?.as_str()?.to_string();
            let dir = f.get("directory")?.as_bool().unwrap_or(false);
            Some((path, dir))
        })
        .collect())
}

async fn stream_get_to(
    client: &Client,
    session_id: &str,
    remote_path: &str,
    dest: &str,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    if dest == "-" {
        let mut stdout = tokio::io::stdout();
        return Ok(client
            .get_file_to(session_id, remote_path, &mut stdout)
            .await?);
    }
    let target = PathBuf::from(dest);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    let mut file = tokio::fs::File::create(&target).await?;
    let n = client
        .get_file_to(session_id, remote_path, &mut file)
        .await?;
    file.flush().await?;
    Ok(n)
}
