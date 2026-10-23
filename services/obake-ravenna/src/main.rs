// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Binary entrypoint: daemon loop and one-shot status client.

use clap::{Parser, Subcommand};
use obake_ravenna::config::Config;
use obake_ravenna::pipewire_nodes::{PipewireClient, RecordingPipewire};
use obake_ravenna::ptp::{interface_from_ptp4l_cfg, PtpManagementClient, PtpState};
use obake_ravenna::qualify::{qualify, QualifyInput};
use obake_ravenna::reconcile::reconcile;
use obake_ravenna::rtsp::{new_shared_table, update_table};
use obake_ravenna::status::{build_report, format_report};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::signal::unix::{signal, SignalKind};

/// Top-level CLI for the `obake-ravenna` binary.
#[derive(Parser, Debug)]
#[command(name = "obake-ravenna", about = "RAVENNA session daemon for PipeWire")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Subcommands: long-running daemon or a status snapshot.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the session daemon (default for the systemd user unit).
    Run {
        /// Path to the stream `config.toml`.
        #[arg(long, default_value = "/home/obake/.config/obake-ravenna/config.toml")]
        config: PathBuf,
        /// Path to `ptp4l` config (used to discover the PTP netdev).
        #[arg(long, default_value = "/home/obake/.config/linuxptp/ptp4l.cfg")]
        ptp4l_config: PathBuf,
        /// `ptp4l` read-only management Unix socket.
        #[arg(long, default_value = "/run/ptp4l/ptp4l.ro")]
        pmc_socket: PathBuf,
    },
    /// Print PTP lock and stream hold-down state.
    Status {
        /// Override `XDG_RUNTIME_DIR` when connecting to the status socket.
        #[arg(long)]
        runtime_dir: Option<PathBuf>,
    },
}

/// Dispatch CLI subcommands.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run {
            config,
            ptp4l_config,
            pmc_socket,
        } => run_daemon(config, ptp4l_config, pmc_socket).await?,
        Commands::Status { runtime_dir } => {
            let dir = runtime_dir
                .or_else(|| std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from))
                .ok_or("XDG_RUNTIME_DIR unset")?;
            status_client(&dir.join("obake-ravenna.status")).await?;
        }
    }
    Ok(())
}

/// Bind the status socket, poll PTP/config, and reconcile RTP modules until exit.
async fn run_daemon(
    config_path: PathBuf,
    ptp4l_config: PathBuf,
    pmc_socket: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR unset")?;
    let status_path = runtime.join("obake-ravenna.status");
    let _ = fs::remove_file(&status_path);
    let listener = UnixListener::bind(&status_path)?;

    let shared_states: Arc<Mutex<Vec<obake_ravenna::qualify::StreamState>>> =
        Arc::new(Mutex::new(Vec::new()));
    let shared_ptp: Arc<Mutex<PtpState>> = Arc::new(Mutex::new(PtpState {
        locked: false,
        domain: 0,
        port_state: obake_ravenna::ptp::PortState::Unknown,
    }));
    let describe = new_shared_table();

    let states_for_status = Arc::clone(&shared_states);
    let ptp_for_status = Arc::clone(&shared_ptp);
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                continue;
            };
            let report = {
                let ptp = ptp_for_status.lock().unwrap();
                let states = states_for_status.lock().unwrap();
                format_report(&build_report(&ptp, &states))
            };
            let _ = sock.write_all(report.as_bytes()).await;
        }
    });

    // The production binary still uses a recording PipeWire client until a
    // real libpipewire binding is wired. PTP lock is read in-process over
    // the management UDS (no pmc spawn). Graph rate is fixed to the
    // host-audio factory value until the PipeWire client can read clock.rate.
    let mut pw = RecordingPipewire::new(48_000);
    let mut current = HashMap::new();
    let client_sock = runtime.join("obake-ravenna.pmc");
    let mut ptp_client = PtpManagementClient::open(&pmc_socket, &client_sock)?;
    let mut sighup = signal(SignalKind::hangup())?;

    loop {
        tick(
            &config_path,
            &ptp4l_config,
            &mut ptp_client,
            &mut pw,
            &mut current,
            &shared_states,
            &shared_ptp,
            &describe,
        )?;

        tokio::select! {
            _ = sighup.recv() => {
                // Reload: re-read config on the next tick without restarting
                // PipeWire.
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    Ok(())
}

/// One control-plane iteration: load config, query PTP, qualify, reconcile, refresh DESCRIBE.
#[allow(clippy::too_many_arguments)]
fn tick(
    config_path: &Path,
    ptp4l_config: &Path,
    ptp_client: &mut PtpManagementClient,
    pw: &mut RecordingPipewire,
    current: &mut HashMap<String, obake_ravenna::pipewire_nodes::ModuleArgs>,
    shared_states: &Arc<Mutex<Vec<obake_ravenna::qualify::StreamState>>>,
    shared_ptp: &Arc<Mutex<PtpState>>,
    describe: &obake_ravenna::rtsp::SharedDescribeTable,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = if config_path.exists() {
        Config::load(config_path)?
    } else {
        Config::default()
    };

    let iface = fs::read_to_string(ptp4l_config)
        .ok()
        .and_then(|t| interface_from_ptp4l_cfg(&t));
    let interface_present = iface
        .as_ref()
        .map(|name| Path::new("/sys/class/net").join(name).exists())
        .unwrap_or(false);

    let ptp = ptp_client.query_state();
    {
        *shared_ptp.lock().unwrap() = ptp;
    }

    let discovered = HashMap::new();
    let states = qualify(&QualifyInput {
        config: &config,
        graph_rate: pw.graph_rate(),
        locked: ptp.locked,
        domain: ptp.domain,
        interface_present,
        discovered: &discovered,
    });

    if let Some(name) = iface.as_deref() {
        reconcile(pw, name, &states, current)?;
    } else {
        reconcile(pw, "", &[], current)?;
    }

    update_table(describe, &states);
    *shared_states.lock().unwrap() = states;
    Ok(())
}

/// Connect to the daemon status socket and print the report to stdout.
async fn status_client(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(path).await?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf).await?;
    print!("{buf}");
    Ok(())
}
