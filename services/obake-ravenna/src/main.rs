// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Binary entrypoint: daemon loop and one-shot status client.

use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{debug, info, warn};
use obake_ravenna::avahi::{
    browse_ravenna, publish_ravenna_node, publish_ravenna_session, AvahiClient, PublishKind,
    RecordingAvahi, SystemAvahi,
};
use obake_ravenna::config::{Config, ConfigError};
use obake_ravenna::pipewire_nodes::{DriverArgs, ModuleArgs, PipewireClient, RecordingPipewire};
use obake_ravenna::ptp::{interface_from_ptp4l_cfg, PtpManagementClient, PtpState};
use obake_ravenna::qualify::{qualify, QualifyInput, StreamState};
use obake_ravenna::reconcile::{reconcile, reconcile_driver};
use obake_ravenna::system_pw::SystemPipewire;
use obake_ravenna::rtsp::{new_shared_table, serve as serve_rtsp, update_table, RTSP_PORT};
use obake_ravenna::status::{build_report, format_report};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;
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
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

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
    info!(
        "startup: daemon starting config={} ptp4l_config={} pmc_socket={}",
        config_path.display(),
        ptp4l_config.display(),
        pmc_socket.display()
    );

    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR unset")?;
    debug!("startup: XDG_RUNTIME_DIR={}", runtime.display());

    let status_path = runtime.join("obake-ravenna.status");
    let _ = fs::remove_file(&status_path);
    let listener = UnixListener::bind(&status_path)?;
    info!("startup: status socket bound at {}", status_path.display());

    let shared_states: Arc<Mutex<Vec<StreamState>>> = Arc::new(Mutex::new(Vec::new()));
    let shared_ptp: Arc<Mutex<PtpState>> = Arc::new(Mutex::new(PtpState::unlocked()));
    let describe = new_shared_table();

    let states_for_status = Arc::clone(&shared_states);
    let ptp_for_status = Arc::clone(&shared_ptp);
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                debug!("status: accept failed; retrying");
                continue;
            };
            debug!("status: client connected");
            let report = {
                let ptp = ptp_for_status.lock().unwrap();
                let states = states_for_status.lock().unwrap();
                format_report(&build_report(&ptp, &states))
            };
            if let Err(e) = sock.write_all(report.as_bytes()).await {
                debug!("status: write failed: {e}");
            }
        }
    });

    let describe_for_rtsp = Arc::clone(&describe);
    tokio::spawn(async move {
        if let Err(e) = serve_rtsp(describe_for_rtsp).await {
            warn!("rtsp: listener stopped: {e}");
        }
    });

    // PipeWire via libpipewire (`pipewire` crate) on the obake user socket;
    // RecordingPipewire if connect fails. PTP lock is read in-process over
    // the management UDS. Media rate comes from [graph].rate.
    let mut avahi: Box<dyn AvahiClient> = match SystemAvahi::connect().await {
        Ok(client) => Box::new(client),
        Err(e) => {
            warn!("mdns: system Avahi unavailable ({e}); using recording stub");
            Box::new(RecordingAvahi::default())
        }
    };
    let mut current: HashMap<String, ModuleArgs> = HashMap::new();
    let mut prev_states: Vec<StreamState> = Vec::new();
    let mut prev_ptp = PtpState::unlocked();
    let mut published_sessions: HashSet<String> = HashSet::new();
    let mut node_published = false;
    let mdns_node_name = mdns_node_name();
    info!("startup: mDNS node name={mdns_node_name}");
    let client_sock = runtime.join("obake-ravenna.pmc");

    let ptp_iface = load_ptp_interface(&ptp4l_config);
    let mut config = initial_stream_config(&config_path);
    let mut pw: Box<dyn PipewireClient> = match SystemPipewire::connect(config.graph.rate) {
        Ok(client) => Box::new(client),
        Err(e) => {
            warn!("pipewire: system client unavailable ({e}); using recording stub");
            Box::new(RecordingPipewire::new(config.graph.rate))
        }
    };
    let mut reload_config = false;
    info!(
        "startup: opening ptp4l management UDS (media rate={})",
        config.graph.rate
    );
    let mut ptp_client = PtpManagementClient::open(&pmc_socket, &client_sock)?;
    let mut sighup = signal(SignalKind::hangup())?;
    info!("startup: entering control loop (tick every 2s; SIGHUP reloads stream config)");

    loop {
        tick(
            &config_path,
            &mut config,
            reload_config,
            &ptp_iface,
            &mut ptp_client,
            pw.as_mut(),
            avahi.as_mut(),
            &mut current,
            &mut prev_states,
            &mut prev_ptp,
            &mdns_node_name,
            &mut node_published,
            &mut published_sessions,
            &shared_states,
            &shared_ptp,
            &describe,
        )?;
        reload_config = false;

        tokio::select! {
            _ = sighup.recv() => {
                info!("signal: SIGHUP received; config will reload on next tick");
                reload_config = true;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                debug!("tick: periodic wake");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("signal: interrupt; shutting down");
                break;
            }
        }
    }

    info!("shutdown: removing Ravenna PipeWire objects");
    let _ = reconcile(pw.as_mut(), "", &[], &mut current);
    let _ = reconcile_driver(pw.as_mut(), None);
    Ok(())
}

/// PTP netdev taken from `ptp4l.cfg` and `/sys/class/net` once at startup.
struct PtpInterface {
    /// Interface name from the non-`global` `ptp4l` section, if any.
    name: Option<String>,
    /// Whether that netdev existed under `/sys/class/net` at startup.
    present: bool,
    /// Kernel ifindex for Avahi publish/browse, when the netdev was present.
    ifindex: Option<i32>,
    /// Unicast IPv4 on that netdev (SDP `o=` / `source-filter`).
    ipv4: Option<Ipv4Addr>,
}

/// Try to read and parse the stream config. `Ok(None)` if the path is absent.
fn try_load_stream_config(path: &Path) -> Result<Option<Config>, ConfigError> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(Config::load(path)?))
}

/// Load stream config once at startup; never abort on missing or invalid TOML.
fn initial_stream_config(path: &Path) -> Config {
    match try_load_stream_config(path) {
        Ok(Some(cfg)) => {
            info!(
                "startup: loaded config path={} sources={} sinks={}",
                path.display(),
                cfg.source.len(),
                cfg.sink.len()
            );
            cfg
        }
        Ok(None) => {
            info!(
                "startup: config missing at {}; using empty defaults",
                path.display()
            );
            Config::default()
        }
        Err(e) => {
            warn!(
                "startup: failed to load config {}: {e}; using empty defaults",
                path.display()
            );
            Config::default()
        }
    }
}

/// Re-read stream config after SIGHUP. On parse/IO failure keep `config` unchanged.
fn reload_stream_config(path: &Path, config: &mut Config) {
    match try_load_stream_config(path) {
        Ok(Some(cfg)) => {
            info!(
                "config: reloaded path={} sources={} sinks={}",
                path.display(),
                cfg.source.len(),
                cfg.sink.len()
            );
            *config = cfg;
        }
        Ok(None) => {
            info!(
                "config: missing at {}; using empty defaults",
                path.display()
            );
            *config = Config::default();
        }
        Err(e) => {
            warn!(
                "config: reload failed for {}: {e}; keeping previous config",
                path.display()
            );
        }
    }
}

/// Read `ptp4l.cfg`, resolve the PTP netdev, and log the result (startup only).
fn load_ptp_interface(ptp4l_config: &Path) -> PtpInterface {
    let name = match fs::read_to_string(ptp4l_config) {
        Ok(text) => {
            let name = interface_from_ptp4l_cfg(&text);
            info!(
                "startup: read ptp4l config path={} interface={:?}",
                ptp4l_config.display(),
                name
            );
            name
        }
        Err(e) => {
            warn!(
                "startup: could not read ptp4l config {}: {e}",
                ptp4l_config.display()
            );
            None
        }
    };
    let (present, ifindex, ipv4) = match name.as_ref() {
        Some(n) => {
            let sys = Path::new("/sys/class/net").join(n);
            let exists = sys.exists();
            let ifindex = if exists {
                read_ifindex(n)
            } else {
                None
            };
            let ipv4 = if exists { read_iface_ipv4(n) } else { None };
            info!(
                "startup: netdev {} syspath={} present={} ifindex={:?} ipv4={:?}",
                n,
                sys.display(),
                exists,
                ifindex,
                ipv4
            );
            (exists, ifindex, ipv4)
        }
        None => {
            info!("startup: no PTP netdev name in ptp4l config; interface_present=false");
            (false, None, None)
        }
    };
    PtpInterface {
        name,
        present,
        ifindex,
        ipv4,
    }
}

/// DNS-SD instance name for this host's RAVENNA **node** (device list entry).
fn mdns_node_name() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "obake".to_string())
}

/// Unicast IPv4 of `name` via `ip -4 -o addr show` (first non-loopback).
fn read_iface_ipv4(name: &str) -> Option<Ipv4Addr> {
    let output = Command::new("ip")
        .args(["-4", "-o", "addr", "show", "dev", name])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.split_whitespace();
    while let Some(tok) = parts.next() {
        if tok != "inet" {
            continue;
        }
        let Some(cidr) = parts.next() else {
            break;
        };
        if let Ok(ip) = cidr
            .split('/')
            .next()
            .unwrap_or(cidr)
            .parse::<Ipv4Addr>()
        {
            if !ip.is_loopback() {
                return Some(ip);
            }
        }
    }
    None
}

/// Read `/sys/class/net/<name>/ifindex` for Avahi interface scoping.
fn read_ifindex(name: &str) -> Option<i32> {
    let path = Path::new("/sys/class/net").join(name).join("ifindex");
    match fs::read_to_string(&path) {
        Ok(text) => match text.trim().parse::<i32>() {
            Ok(idx) if idx > 0 => Some(idx),
            Ok(idx) => {
                warn!("startup: ignoring non-positive ifindex {idx} from {}", path.display());
                None
            }
            Err(e) => {
                warn!("startup: parse ifindex from {}: {e}", path.display());
                None
            }
        },
        Err(e) => {
            warn!("startup: read {}: {e}", path.display());
            None
        }
    }
}

/// One control-plane iteration: optional config reload, PTP query, qualify, reconcile, DESCRIBE.
#[allow(clippy::too_many_arguments)]
fn tick(
    config_path: &Path,
    config: &mut Config,
    reload_config: bool,
    ptp_iface: &PtpInterface,
    ptp_client: &mut PtpManagementClient,
    pw: &mut dyn PipewireClient,
    avahi: &mut dyn AvahiClient,
    current: &mut HashMap<String, ModuleArgs>,
    prev_states: &mut Vec<StreamState>,
    prev_ptp: &mut PtpState,
    mdns_node_name: &str,
    node_published: &mut bool,
    published_sessions: &mut HashSet<String>,
    shared_states: &Arc<Mutex<Vec<StreamState>>>,
    shared_ptp: &Arc<Mutex<PtpState>>,
    describe: &obake_ravenna::rtsp::SharedDescribeTable,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("tick: begin");

    if reload_config {
        reload_stream_config(config_path, config);
    }

    debug!("tick: querying ptp4l management socket");
    let ptp = ptp_client.query_state();
    if ptp.locked != prev_ptp.locked
        || ptp.domain != prev_ptp.domain
        || ptp.port_state != prev_ptp.port_state
        || ptp.gmid != prev_ptp.gmid
    {
        info!(
            "ptp: state change locked {} -> {} domain {} -> {} port_state {} -> {} gmid {:?} -> {:?}",
            prev_ptp.locked,
            ptp.locked,
            prev_ptp.domain,
            ptp.domain,
            prev_ptp.port_state.as_str(),
            ptp.port_state.as_str(),
            prev_ptp.gmid,
            ptp.gmid
        );
    }
    *prev_ptp = ptp;
    {
        *shared_ptp.lock().unwrap() = ptp;
    }

    let browse_ifindex = ptp_iface.ifindex.unwrap_or(-1);
    let browse_req = browse_ravenna(browse_ifindex);
    if let Err(e) = avahi.browse(&browse_req) {
        warn!("mdns: browse failed: {e}");
    }
    let discovered = HashMap::new();
    debug!("tick: discovery results count={}", discovered.len());

    debug!(
        "tick: qualify rate={} frames={} locked={} gmid={:?}",
        config.graph.rate, config.packet.frames, ptp.locked, ptp.gmid
    );
    // Refresh IPv4 each tick so DHCP renewals land in SDP without restart.
    let source_ip = ptp_iface
        .name
        .as_deref()
        .filter(|_| ptp_iface.present)
        .and_then(read_iface_ipv4)
        .or(ptp_iface.ipv4);
    let states = qualify(&QualifyInput {
        config,
        locked: ptp.locked,
        domain: ptp.domain,
        gmid: ptp.gmid,
        interface_present: ptp_iface.present,
        source_ip,
        discovered: &discovered,
    });
    for state in &states {
        debug!("tick: qualify {}", state.status_label());
    }
    log_stream_state_changes(prev_states, &states);

    // PTP driver while the interface exists; RTP nodes only when streams qualify.
    if let Some(iface) = ptp_iface.name.as_deref().filter(|_| ptp_iface.present) {
        let driver = DriverArgs {
            name: "obake-ptp-driver".into(),
            node_group: config.graph.node_group.clone(),
            clock_interface: iface.to_string(),
        };
        reconcile_driver(pw, Some(&driver))?;
        debug!("tick: reconcile streams on iface={iface}");
        reconcile(pw, iface, &states, current)?;
    } else {
        debug!("tick: no iface; unloading streams and driver");
        reconcile(pw, "", &[], current)?;
        reconcile_driver(pw, None)?;
    }

    sync_mdns_registration(
        avahi,
        ptp_iface.ifindex,
        mdns_node_name,
        node_published,
        published_sessions,
        &states,
    );

    update_table(describe, &states);
    debug!(
        "tick: DESCRIBE table refreshed; active modules={}",
        current.len()
    );
    *shared_states.lock().unwrap() = states.clone();
    *prev_states = states;
    debug!("tick: end");
    Ok(())
}

/// Log info lines when a stream's up/down or [`DownReason`] changes.
fn log_stream_state_changes(prev: &[StreamState], next: &[StreamState]) {
    let prev_map: HashMap<(&str, &str), &StreamState> = prev
        .iter()
        .map(|s| ((s.kind(), s.name()), s))
        .collect();
    let next_keys: HashSet<(&str, &str)> = next.iter().map(|s| (s.kind(), s.name())).collect();

    for state in next {
        let key = (state.kind(), state.name());
        match prev_map.get(&key) {
            None => info!("stream: new {}", state.status_label()),
            Some(old) if *old != state => {
                info!(
                    "stream: {} -> {}",
                    old.status_label(),
                    state.status_label()
                );
            }
            _ => {}
        }
    }
    for state in prev {
        let key = (state.kind(), state.name());
        if !next_keys.contains(&key) {
            info!("stream: removed {}", state.status_label());
        }
    }
}

/// Publish node + session mDNS records per RAVENNA §3.5.
fn sync_mdns_registration(
    avahi: &mut dyn AvahiClient,
    ifindex: Option<i32>,
    node_name: &str,
    node_published: &mut bool,
    published_sessions: &mut HashSet<String>,
    states: &[StreamState],
) {
    let want_sessions: HashSet<String> = states
        .iter()
        .filter_map(|s| match s {
            StreamState::SourceUp(src) => Some(src.name.clone()),
            _ => None,
        })
        .collect();

    let Some(ifindex) = ifindex else {
        if !want_sessions.is_empty() || *node_published {
            warn!("mdns: no PTP ifindex; withdrawing advertisements");
        }
        withdraw_all_mdns(avahi, node_name, node_published, published_sessions);
        return;
    };

    // Device lists browse `_ravenna._sub._rtsp._tcp` for the node, not sessions.
    if !*node_published {
        let req = publish_ravenna_node(node_name, RTSP_PORT, ifindex);
        match avahi.publish(&req) {
            Ok(()) => {
                info!("mdns: registered node={node_name}");
                *node_published = true;
            }
            Err(e) => warn!("mdns: register node={node_name} failed: {e}"),
        }
    }

    let to_add: Vec<String> = want_sessions
        .difference(published_sessions)
        .cloned()
        .collect();
    for name in to_add {
        let req = publish_ravenna_session(&name, RTSP_PORT, ifindex);
        match avahi.publish(&req) {
            Ok(()) => {
                info!("mdns: registered session={name}");
                published_sessions.insert(name);
            }
            Err(e) => warn!("mdns: register session={name} failed: {e}"),
        }
    }
    let to_drop: Vec<String> = published_sessions
        .difference(&want_sessions)
        .cloned()
        .collect();
    for name in to_drop {
        match avahi.unpublish(PublishKind::Session, &name) {
            Ok(()) => info!("mdns: unregistered session={name}"),
            Err(e) => warn!("mdns: unregister session={name} failed: {e}"),
        }
        published_sessions.remove(&name);
    }
}

/// Withdraw node and all session advertisements.
fn withdraw_all_mdns(
    avahi: &mut dyn AvahiClient,
    node_name: &str,
    node_published: &mut bool,
    published_sessions: &mut HashSet<String>,
) {
    let sessions: Vec<String> = published_sessions.drain().collect();
    for name in sessions {
        if let Err(e) = avahi.unpublish(PublishKind::Session, &name) {
            warn!("mdns: unpublish session={name} failed: {e}");
        }
    }
    if *node_published {
        if let Err(e) = avahi.unpublish(PublishKind::Node, node_name) {
            warn!("mdns: unpublish node={node_name} failed: {e}");
        }
        *node_published = false;
    }
}

/// Connect to the daemon status socket and print the report to stdout.
async fn status_client(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    debug!("status: connecting to {}", path.display());
    let mut stream = UnixStream::connect(path).await?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf).await?;
    print!("{buf}");
    Ok(())
}
