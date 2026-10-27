// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Session lifecycle: start/stop/restore, files, links, rigs.

use crate::config::{
    load_rigs, load_rig, load_host, parse_rig, Rig, HostProfile, RoleDirection,
    RoleRef,
};
use crate::launch::{
    pipewire_props, singularity_name, BindMount, LaunchRequest, Launcher,
};
use crate::links::{GraphOps, LinkState, LinkStatus, Reconciler};
use crate::paths::Paths;
use crate::shapes::{self, is_launchable, ShapeInfo};
use crate::auth::LoginBook;
use crate::store::{
    self, copy_dir_all, delete_session_file, ensure_token, list_sessions, read_current,
    read_session, write_current, write_session, write_token, RoleRefSerde, SessionMode,
    SessionRecord, SystemEndpoint, SystemLink,
};
use crate::tls::ensure_host_cert;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

/// Service-level errors surfaced to GraphQL.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Generic message.
    #[error("{0}")]
    Message(String),
}

impl ServiceError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

/// Runtime session phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    /// Instances starting or links pending.
    Provisioning,
    /// All instances up and desired links applied.
    Linked,
    /// Stopping.
    TearingDown,
}

/// Live view of the active session.
#[derive(Debug, Clone)]
pub struct ActiveSession {
    /// Durable record.
    pub record: SessionRecord,
    /// Runtime phase.
    pub phase: SessionPhase,
    /// Per-link status.
    pub link_status: Vec<LinkStatus>,
    /// Running singularity names.
    pub running: Vec<String>,
}

/// Session work-area file listing entry.
#[derive(Debug, Clone)]
pub struct SessionFileEntry {
    /// Relative path.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// True if directory.
    pub directory: bool,
}

/// Shared service state.
pub struct Service {
    paths: Paths,
    host: HostProfile,
    image_dir: PathBuf,
    launcher: Arc<dyn Launcher>,
    graph: Arc<dyn GraphOps>,
    inner: Mutex<Inner>,
    token: RwLock<String>,
    fingerprint: RwLock<String>,
    listen_port: AtomicU16,
    logins: Mutex<LoginBook>,
    ravenna_available: bool,
}

struct Inner {
    active: Option<ActiveSession>,
    reconciler: Reconciler,
    session_tx: tokio::sync::broadcast::Sender<Option<ActiveSession>>,
    links_tx: tokio::sync::broadcast::Sender<Vec<LinkStatus>>,
}

impl Service {
    /// Build a service with injected launcher and graph (for tests / production).
    pub fn new(
        paths: Paths,
        launcher: Arc<dyn Launcher>,
        graph: Arc<dyn GraphOps>,
    ) -> Result<Arc<Self>, ServiceError> {
        paths
            .ensure()
            .map_err(|e| ServiceError::msg(e.to_string()))?;
        let host = load_host(&paths.host_yaml()).map_err(|e| ServiceError::msg(e.to_string()))?;
        let image_dir = host
            .image_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| paths.default_image_dir.clone());
        let token = ensure_token(&paths).map_err(|e| ServiceError::msg(e.to_string()))?;
        let (_cfg, fingerprint) =
            ensure_host_cert(&paths).map_err(|e| ServiceError::msg(e.to_string()))?;
        let listen_port = host
            .listen
            .parse::<SocketAddr>()
            .map(|a| a.port())
            .unwrap_or(9477);
        let ravenna_available = paths.ravenna_config_dir.join("config.toml").is_file();
        let (session_tx, _) = tokio::sync::broadcast::channel(64);
        let (links_tx, _) = tokio::sync::broadcast::channel(64);
        let svc = Arc::new(Self {
            paths,
            host,
            image_dir,
            launcher,
            graph,
            inner: Mutex::new(Inner {
                active: None,
                reconciler: Reconciler::new(),
                session_tx,
                links_tx,
            }),
            token: RwLock::new(token),
            fingerprint: RwLock::new(fingerprint),
            listen_port: AtomicU16::new(listen_port),
            logins: Mutex::new(LoginBook::new()),
            ravenna_available,
        });
        Ok(svc)
    }

    /// Paths in use.
    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    /// Host profile.
    pub fn host(&self) -> &HostProfile {
        &self.host
    }

    /// Reload host.yaml from disk.
    pub fn reload_host(&mut self) -> Result<(), ServiceError> {
        self.host = load_host(&self.paths.host_yaml()).map_err(|e| ServiceError::msg(e.to_string()))?;
        self.image_dir = self
            .host
            .image_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.paths.default_image_dir.clone());
        Ok(())
    }

    /// Current host bearer.
    pub fn token(&self) -> String {
        self.token.read().expect("token lock").clone()
    }

    /// Replace the bearer in memory and on disk. The certificate is unchanged.
    pub fn rotate_token(&self) -> Result<String, ServiceError> {
        let token = Uuid::new_v4().to_string();
        write_token(&self.paths, &token).map_err(|e| ServiceError::msg(e.to_string()))?;
        *self.token.write().expect("token lock") = token.clone();
        Ok(token)
    }

    /// SHA-256 fingerprint of the TLS certificate (`sha256:` + hex).
    pub fn fingerprint(&self) -> String {
        self.fingerprint.read().expect("fingerprint lock").clone()
    }

    /// TCP port of the TLS listener.
    pub fn listen_port(&self) -> u16 {
        self.listen_port.load(Ordering::Relaxed)
    }

    /// Record the port actually bound (ephemeral test listeners).
    pub fn set_listen_port(&self, port: u16) {
        self.listen_port.store(port, Ordering::Relaxed);
    }

    /// Begin a browser device login.
    pub fn start_login(&self) -> Result<crate::auth::LoginStart, ServiceError> {
        self.logins
            .lock()
            .expect("login lock")
            .start()
            .map_err(ServiceError::msg)
    }

    /// Poll a device login. `Ok(true)` means the cookie may be set.
    pub fn poll_login(&self, poll_secret: &str) -> Result<bool, ServiceError> {
        self.logins
            .lock()
            .expect("login lock")
            .poll(poll_secret)
            .map_err(ServiceError::msg)
    }

    /// Mark a device code approved.
    pub fn approve_login(&self, code: &str) -> Result<(), ServiceError> {
        self.logins
            .lock()
            .expect("login lock")
            .approve(code)
            .map_err(ServiceError::msg)
    }

    /// Age pending logins (tests).
    pub fn backdate_pending_logins(&self, by: Duration) {
        self.logins.lock().expect("login lock").backdate(by);
    }

    /// Image directory.
    pub fn image_dir(&self) -> &Path {
        &self.image_dir
    }

    /// Subscribe to session changes.
    pub fn subscribe_session(&self) -> tokio::sync::broadcast::Receiver<Option<ActiveSession>> {
        self.inner.lock().unwrap().session_tx.subscribe()
    }

    /// Subscribe to link status changes.
    pub fn subscribe_links(&self) -> tokio::sync::broadcast::Receiver<Vec<LinkStatus>> {
        self.inner.lock().unwrap().links_tx.subscribe()
    }

    fn publish_session(&self, active: &Option<ActiveSession>) {
        let _ = self
            .inner
            .lock()
            .unwrap()
            .session_tx
            .send(active.clone());
    }

    fn publish_links(&self, links: &[LinkStatus]) {
        let _ = self.inner.lock().unwrap().links_tx.send(links.to_vec());
    }

    /// Restore from `current` on boot.
    pub fn restore(&self) -> Result<(), ServiceError> {
        let current = read_current(&self.paths).map_err(|e| ServiceError::msg(e.to_string()))?;
        let Some(id) = current else {
            return Ok(());
        };
        let record = read_session(&self.paths, &id).map_err(|e| ServiceError::msg(e.to_string()))?;
        self.launch_record(record, true)
    }

    /// List installed rigs.
    pub fn rigs(&self) -> Result<Vec<Rig>, ServiceError> {
        load_rigs(&self.paths.rigs_dir()).map_err(|e| ServiceError::msg(e.to_string()))
    }

    /// One rig by name.
    pub fn rig(&self, name: &str) -> Result<Option<Rig>, ServiceError> {
        let pkg = self
            .paths
            .resolve_rig(name)
            .map_err(|_| ServiceError::msg(format!(
                "rig `{name}` has both `<name>.yaml` and `<name>/rig.yaml`"
            )))?;
        let Some(pkg) = pkg else {
            return Ok(None);
        };
        Ok(Some(
            load_rig(name, pkg.definition()).map_err(|e| ServiceError::msg(e.to_string()))?,
        ))
    }

    /// Compiled shapes with image presence.
    pub fn shapes(&self) -> Vec<ShapeInfo> {
        shapes::list_shapes(&self.image_dir)
    }

    /// Active session snapshot.
    pub fn session(&self) -> Option<ActiveSession> {
        self.inner.lock().unwrap().active.clone()
    }

    /// Recorded sessions, optionally filtered by rig.
    pub fn sessions(&self, rig: Option<&str>) -> Result<Vec<SessionRecord>, ServiceError> {
        let mut all = list_sessions(&self.paths).map_err(|e| ServiceError::msg(e.to_string()))?;
        if let Some(name) = rig {
            all.retain(|s| s.rig == name);
        }
        Ok(all)
    }

    /// Start a fresh or forked session.
    pub fn start_session(
        &self,
        rig_name: &str,
        mode: SessionMode,
        fork_from: Option<&str>,
    ) -> Result<ActiveSession, ServiceError> {
        let rig = self
            .rig(rig_name)?
            .ok_or_else(|| ServiceError::msg(format!("unknown rig `{rig_name}`")))?;

        self.validate_launchable(&rig)?;

        let (id, forked_from, system_links) = match mode {
            SessionMode::Fresh => (Uuid::new_v4().to_string(), None, Vec::new()),
            SessionMode::Fork => {
                let src = fork_from
                    .ok_or_else(|| ServiceError::msg("fork requires forkFrom"))?;
                let src_rec =
                    read_session(&self.paths, src).map_err(|e| ServiceError::msg(e.to_string()))?;
                if src_rec.rig != rig_name {
                    return Err(ServiceError::msg(
                        "fork source rig does not match requested rig",
                    ));
                }
                (
                    Uuid::new_v4().to_string(),
                    Some(src.to_string()),
                    src_rec.system_links.clone(),
                )
            }
        };

        // Prepare session work before stopping the previous session.
        let session_work = self.paths.session_work(&id);
        fs::create_dir_all(&session_work).map_err(|e| ServiceError::msg(e.to_string()))?;

        match mode {
            SessionMode::Fresh => {
                self.seed_session_work(rig_name, &session_work)?;
            }
            SessionMode::Fork => {
                let src = fork_from.unwrap();
                let src_work = self.paths.session_work(src);
                if src_work.exists() {
                    copy_dir_all(&src_work, &session_work)
                        .map_err(|e| ServiceError::msg(e.to_string()))?;
                }
            }
        }

        let record = SessionRecord {
            id: id.clone(),
            rig: rig_name.into(),
            mode,
            forked_from,
            system_links,
        };
        write_session(&self.paths, &record).map_err(|e| ServiceError::msg(e.to_string()))?;

        // Stop previous after validation succeeded.
        self.stop_instances_only()?;

        write_current(&self.paths, Some(&id)).map_err(|e| ServiceError::msg(e.to_string()))?;
        self.launch_record(record, false)?;
        self.session()
            .ok_or_else(|| ServiceError::msg("session missing after start"))
    }

    fn validate_launchable(&self, rig: &Rig) -> Result<(), ServiceError> {
        for inst in rig.instances.values() {
            if !is_launchable(&inst.shape) {
                return Err(ServiceError::msg(format!(
                    "shape `{}` is not launchable",
                    inst.shape
                )));
            }
            if !shapes::image_present(&self.image_dir, &inst.shape) {
                return Err(ServiceError::msg(format!(
                    "image for shape `{}` is not present",
                    inst.shape
                )));
            }
        }
        Ok(())
    }

    fn stop_instances_only(&self) -> Result<(), ServiceError> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(active) = inner.active.take() {
            for name in &active.running {
                let _ = self.launcher.stop(name);
            }
            inner.reconciler.clear();
        }
        Ok(())
    }

    fn launch_record(&self, record: SessionRecord, _restoring: bool) -> Result<(), ServiceError> {
        let rig = self
            .rig(&record.rig)?
            .ok_or_else(|| ServiceError::msg(format!("rig `{}` missing", record.rig)))?;

        let session_work = self.paths.session_work(&record.id);
        fs::create_dir_all(&session_work).map_err(|e| ServiceError::msg(e.to_string()))?;

        let mut running = Vec::new();
        let mut instances: Vec<_> = rig.instances.values().collect();
        instances.sort_by_key(|i| &i.name);

        for inst in instances {
            let req = self.build_launch(&record, inst)?;
            self.launcher
                .start(&req)
                .map_err(|e| ServiceError::msg(e.to_string()))?;
            running.push(req.singularity_name);
        }

        let active = ActiveSession {
            record: record.clone(),
            phase: SessionPhase::Provisioning,
            link_status: Vec::new(),
            running,
        };
        {
            let mut inner = self.inner.lock().unwrap();
            inner.active = Some(active.clone());
        }
        self.publish_session(&Some(active));
        self.reconcile_links()?;
        Ok(())
    }

    fn build_launch(
        &self,
        record: &SessionRecord,
        inst: &crate::config::Instance,
    ) -> Result<LaunchRequest, ServiceError> {
        let image = self.paths.shape_image(&self.image_dir, &inst.shape);
        let mut env = self.host.environment.clone();
        for (k, v) in &inst.environment {
            env.insert(k.clone(), v.clone());
        }
        env.insert(
            "XDG_RUNTIME_DIR".into(),
            self.paths.runtime_dir.display().to_string(),
        );
        env.insert(
            "PIPEWIRE_PROPS".into(),
            pipewire_props(&inst.node_name),
        );

        let mut binds = Vec::new();
        // Runtime dir bind
        binds.push(BindMount {
            source: self.paths.runtime_dir.clone(),
            target: self.paths.runtime_dir.display().to_string(),
            readonly: false,
        });
        binds.push(BindMount {
            source: self.paths.session_work(&record.id),
            target: "/work".into(),
            readonly: false,
        });
        for b in &self.host.binds {
            binds.push(BindMount {
                source: PathBuf::from(&b.source),
                target: b.target.clone(),
                readonly: b.readonly,
            });
        }
        if let Some(lib) = &self.host.sample_library {
            binds.push(BindMount {
                source: PathBuf::from(&lib.source),
                target: lib.target.clone(),
                readonly: true,
            });
        }

        Ok(LaunchRequest {
            singularity_name: singularity_name(&record.id, &inst.name),
            instance_name: inst.name.clone(),
            shape: inst.shape.clone(),
            image,
            command: inst.command.clone(),
            env,
            binds,
        })
    }

    /// Stop the active session and clear `current`.
    pub fn stop_session(&self) -> Result<bool, ServiceError> {
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(mut active) = inner.active.take() {
                active.phase = SessionPhase::TearingDown;
                for name in &active.running {
                    let _ = self.launcher.stop(name);
                }
                // Disconnect applied links
                for st in &active.link_status {
                    if st.state == LinkState::Applied {
                        let _ = self.graph.disconnect(&st.from, &st.to);
                    }
                }
                inner.reconciler.clear();
            }
        }
        write_current(&self.paths, None).map_err(|e| ServiceError::msg(e.to_string()))?;
        self.publish_session(&None);
        self.publish_links(&[]);
        Ok(true)
    }

    /// Replace system links on the active session.
    pub fn set_system_links(&self, links: Vec<SystemLink>) -> Result<ActiveSession, ServiceError> {
        let record = {
            let inner = self.inner.lock().unwrap();
            let active = inner
                .active
                .as_ref()
                .ok_or_else(|| ServiceError::msg("no active session"))?;
            let mut record = active.record.clone();
            record.system_links = links;
            record
        };
        write_session(&self.paths, &record).map_err(|e| ServiceError::msg(e.to_string()))?;
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(active) = inner.active.as_mut() {
                active.record = record;
            }
        }
        self.reconcile_links()?;
        self.session()
            .ok_or_else(|| ServiceError::msg("no active session"))
    }

    /// Reconcile rig + system links against the live graph.
    pub fn reconcile_links(&self) -> Result<Vec<LinkStatus>, ServiceError> {
        let (rig_name, system_links, record_id) = {
            let inner = self.inner.lock().unwrap();
            let Some(active) = inner.active.as_ref() else {
                return Ok(Vec::new());
            };
            (
                active.record.rig.clone(),
                active.record.system_links.clone(),
                active.record.id.clone(),
            )
        };
        let rig = self
            .rig(&rig_name)?
            .ok_or_else(|| ServiceError::msg("rig missing"))?;

        let mut desired = Vec::new();
        for link in &rig.links {
            if let Some(pair) = self.resolve_role_link(&rig, &link.from, &link.to) {
                desired.push(pair);
            }
        }
        for link in &system_links {
            let from_ref = RoleRef::from(&link.from);
            if let Some(from_ep) = self.resolve_role_endpoint(&rig, &from_ref) {
                if let Some(to_ep) = self.resolve_system_endpoint(&link.to) {
                    desired.push((from_ep, to_ep));
                } else {
                    // Keep as pending with unresolved alias target text.
                    let to = format_system_endpoint(&link.to);
                    desired.push((from_ep, to));
                }
            }
        }

        let status = {
            let mut inner = self.inner.lock().unwrap();
            let status = inner
                .reconciler
                .reconcile(self.graph.as_ref(), &desired)
                .map_err(|e| ServiceError::msg(e.to_string()))?;
            if let Some(active) = inner.active.as_mut() {
                active.link_status = status.clone();
                let all_applied = status.iter().all(|s| s.state == LinkState::Applied);
                active.phase = if all_applied {
                    SessionPhase::Linked
                } else {
                    SessionPhase::Provisioning
                };
            }
            let _ = record_id;
            status
        };
        let active = self.session();
        self.publish_session(&active);
        self.publish_links(&status);
        Ok(status)
    }

    fn resolve_role_link(
        &self,
        rig: &Rig,
        from: &RoleRef,
        to: &RoleRef,
    ) -> Option<(String, String)> {
        let a = self.resolve_role_endpoint(rig, from)?;
        let b = self.resolve_role_endpoint(rig, to)?;
        Some((a, b))
    }

    fn resolve_role_endpoint(&self, rig: &Rig, role_ref: &RoleRef) -> Option<String> {
        let inst = rig.instances.get(&role_ref.instance)?;
        let role = inst.roles.iter().find(|r| r.name == role_ref.role)?;
        shapes::endpoint(&inst.node_name, &inst.shape, role.direction, role.index)
    }

    fn resolve_system_endpoint(&self, ep: &SystemEndpoint) -> Option<String> {
        if let Some(node) = &ep.node {
            return Some(format!("{}:{}", node, ep.port));
        }
        if let Some(alias) = &ep.alias {
            let card = self.host.aliases.get(alias)?.card.as_str();
            let node = self.graph.resolve_card(card)?;
            return Some(format!("{node}:{}", ep.port));
        }
        None
    }

    /// List files in a session work area.
    pub fn session_files(&self, id: &str) -> Result<Vec<SessionFileEntry>, ServiceError> {
        let root = self.paths.session_work(id);
        if !root.exists() {
            return Err(ServiceError::msg("session work area not found"));
        }
        let mut out = Vec::new();
        list_files_rec(&root, &root, &mut out)?;
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    /// Resolve and open a work-area file for streaming read.
    pub fn open_session_file(
        &self,
        id: &str,
        path: &str,
    ) -> Result<(File, u64), ServiceError> {
        let full = self.session_file_path(id, path)?;
        if full.is_dir() {
            return Err(ServiceError::msg("path is a directory"));
        }
        let file = File::open(&full).map_err(|e| ServiceError::msg(e.to_string()))?;
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok((file, len))
    }

    /// Read one work-area file as raw bytes (small files / tests).
    pub fn read_session_file(&self, id: &str, path: &str) -> Result<Vec<u8>, ServiceError> {
        let (mut file, _) = self.open_session_file(id, path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| ServiceError::msg(e.to_string()))?;
        Ok(bytes)
    }

    /// Stream a reader into a work-area file (temp sibling, then rename).
    pub fn write_session_file_from_reader(
        &self,
        id: &str,
        path: &str,
        reader: &mut dyn Read,
    ) -> Result<(), ServiceError> {
        let full = self.session_file_path_for_write(id, path)?;
        store::atomic_write_from_reader(&full, reader)
            .map_err(|e| ServiceError::msg(e.to_string()))
    }

    /// Write (create or replace) a regular file in a session work area.
    pub fn write_session_file(
        &self,
        id: &str,
        path: &str,
        bytes: &[u8],
    ) -> Result<(), ServiceError> {
        self.write_session_file_from_reader(id, path, &mut io::Cursor::new(bytes))
    }

    fn session_file_path(&self, id: &str, path: &str) -> Result<PathBuf, ServiceError> {
        let root = self.paths.session_work(id);
        let rel = sanitize_rel_path(path)?;
        let full = root.join(&rel);
        if !full.starts_with(&root) {
            return Err(ServiceError::msg("path escapes session work area"));
        }
        Ok(full)
    }

    /// Resolve a writable work-area path (creates parent dirs).
    pub fn session_file_path_for_write(
        &self,
        id: &str,
        path: &str,
    ) -> Result<PathBuf, ServiceError> {
        let root = self.paths.session_work(id);
        if !root.exists() {
            return Err(ServiceError::msg("session work area not found"));
        }
        let full = self.session_file_path(id, path)?;
        if full.exists() && full.is_dir() {
            return Err(ServiceError::msg("cannot overwrite a directory"));
        }
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).map_err(|e| ServiceError::msg(e.to_string()))?;
        }
        Ok(full)
    }

    /// Resolve `current` to the active session id.
    pub fn resolve_session_id(&self, id_or_current: &str) -> Result<String, ServiceError> {
        if id_or_current == "current" {
            self.session()
                .map(|s| s.record.id)
                .ok_or_else(|| ServiceError::msg("no active session"))
        } else {
            Ok(id_or_current.to_string())
        }
    }

    /// Delete an inactive session.
    pub fn delete_session(&self, id: &str) -> Result<bool, ServiceError> {
        if let Some(active) = self.session() {
            if active.record.id == id {
                return Err(ServiceError::msg("cannot delete the active session"));
            }
        }
        delete_session_file(&self.paths, id).map_err(|e| ServiceError::msg(e.to_string()))?;
        let work = self.paths.session_work(id);
        if work.exists() {
            // Remove the session/<id> directory (parent of work).
            if let Some(parent) = work.parent() {
                fs::remove_dir_all(parent).map_err(|e| ServiceError::msg(e.to_string()))?;
            }
        }
        Ok(true)
    }

    /// Copy the package `work/` template into a new session work directory.
    fn seed_session_work(&self, rig_name: &str, session_work: &Path) -> Result<(), ServiceError> {
        let pkg = self
            .paths
            .resolve_rig(rig_name)
            .map_err(|_| ServiceError::msg(format!(
                "rig `{rig_name}` has both `<name>.yaml` and `<name>/rig.yaml`"
            )))?
            .ok_or_else(|| ServiceError::msg(format!("unknown rig `{rig_name}`")))?;
        let Some(template) = pkg.work_template() else {
            return Ok(());
        };
        if !template.is_dir() {
            return Ok(());
        }
        copy_dir_all(template, session_work).map_err(|e| ServiceError::msg(e.to_string()))
    }

    /// Clear whichever package form exists for `name`.
    fn clear_rig_package(&self, name: &str) -> Result<(), ServiceError> {
        let file = self.paths.rig_file(name);
        if file.exists() {
            fs::remove_file(&file).map_err(|e| ServiceError::msg(e.to_string()))?;
        }
        let dir = self.paths.rig_dir(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| ServiceError::msg(e.to_string()))?;
        }
        Ok(())
    }

    /// Install a single-file rig document (YAML).
    pub fn install_rig(&self, name: &str, document: &str) -> Result<Rig, ServiceError> {
        let rig =
            parse_rig(name, document).map_err(|e| ServiceError::msg(e.to_string()))?;
        fs::create_dir_all(self.paths.rigs_dir())
            .map_err(|e| ServiceError::msg(e.to_string()))?;
        self.clear_rig_package(name)?;
        let path = self.paths.rig_file(name);
        store::atomic_write(&path, document.as_bytes())
            .map_err(|e| ServiceError::msg(e.to_string()))?;
        Ok(rig)
    }

    /// Install a directory package already extracted under a temporary root.
    ///
    /// `src` must contain `rig.yaml` and may contain `work/`.
    pub fn install_rig_dir(&self, name: &str, src: &Path) -> Result<Rig, ServiceError> {
        let def = src.join("rig.yaml");
        if !def.is_file() {
            return Err(ServiceError::msg("package missing rig.yaml"));
        }
        let text = fs::read_to_string(&def).map_err(|e| ServiceError::msg(e.to_string()))?;
        let rig = parse_rig(name, &text).map_err(|e| ServiceError::msg(e.to_string()))?;
        fs::create_dir_all(self.paths.rigs_dir())
            .map_err(|e| ServiceError::msg(e.to_string()))?;
        self.clear_rig_package(name)?;
        let dest = self.paths.rig_dir(name);
        copy_dir_all(src, &dest).map_err(|e| ServiceError::msg(e.to_string()))?;
        Ok(rig)
    }

    /// Install a directory package from an uncompressed tar archive.
    pub fn install_rig_tar(&self, name: &str, archive: &[u8]) -> Result<Rig, ServiceError> {
        let tmp = tempfile::tempdir().map_err(|e| ServiceError::msg(e.to_string()))?;
        extract_package_tar(archive, tmp.path())?;
        self.install_rig_dir(name, tmp.path())
    }

    /// Remove a rig; fails if it is the active session's rig.
    pub fn remove_rig(&self, name: &str) -> Result<bool, ServiceError> {
        if let Some(active) = self.session() {
            if active.record.rig == name {
                return Err(ServiceError::msg(
                    "cannot remove the rig of the active session",
                ));
            }
        }
        self.clear_rig_package(name)?;
        Ok(true)
    }

    /// Live graph nodes/ports from the reconciler's view.
    pub fn graph(&self) -> Vec<(String, Vec<(String, RoleDirection)>)> {
        let ports = self.graph.list_ports();
        let mut map: HashMap<String, Vec<(String, RoleDirection)>> = HashMap::new();
        for p in ports {
            let dir = match p.direction {
                crate::links::PortDirection::In => RoleDirection::In,
                crate::links::PortDirection::Out => RoleDirection::Out,
                crate::links::PortDirection::Unknown => RoleDirection::Out,
            };
            map.entry(p.node).or_default().push((p.port, dir));
        }
        let mut nodes: Vec<_> = map.into_iter().collect();
        nodes.sort_by(|a, b| a.0.cmp(&b.0));
        nodes
    }

    /// Replace Ravenna sources/sinks and reload (milestone 5).
    pub fn set_ravenna_streams(
        &self,
        sources: &[(String, u32, String)],
        sinks: &[(String, String, String)],
    ) -> Result<bool, ServiceError> {
        if !self.ravenna_available {
            return Err(ServiceError::msg(
                "obake-ravenna is not installed on this host",
            ));
        }
        let config_path = self.paths.ravenna_config_dir.join("config.toml");
        let text = fs::read_to_string(&config_path).map_err(|e| ServiceError::msg(e.to_string()))?;
        let mut doc: toml::Value =
            toml::from_str(&text).map_err(|e| ServiceError::msg(e.to_string()))?;

        let sources_val: Vec<toml::Value> = sources
            .iter()
            .map(|(name, channels, format)| {
                let mut t = toml::map::Map::new();
                t.insert("name".into(), toml::Value::String(name.clone()));
                t.insert(
                    "channels".into(),
                    toml::Value::Integer(*channels as i64),
                );
                t.insert("format".into(), toml::Value::String(format.clone()));
                toml::Value::Table(t)
            })
            .collect();
        let sinks_val: Vec<toml::Value> = sinks
            .iter()
            .map(|(name, device, stream)| {
                let mut t = toml::map::Map::new();
                t.insert("name".into(), toml::Value::String(name.clone()));
                t.insert("device".into(), toml::Value::String(device.clone()));
                t.insert("stream".into(), toml::Value::String(stream.clone()));
                toml::Value::Table(t)
            })
            .collect();

        if let toml::Value::Table(ref mut table) = doc {
            table.insert("sources".into(), toml::Value::Array(sources_val));
            table.insert("sinks".into(), toml::Value::Array(sinks_val));
        }
        let out = toml::to_string_pretty(&doc).map_err(|e| ServiceError::msg(e.to_string()))?;
        store::atomic_write(&config_path, out.as_bytes())
            .map_err(|e| ServiceError::msg(e.to_string()))?;

        // Reload user service without restarting PipeWire.
        if std::env::var_os("OBAKE_SESSION_SKIP_RAVENNA_RELOAD").is_none() {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "reload", "obake-ravenna.service"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        Ok(true)
    }

    /// Mark Ravenna available (tests).
    pub fn set_ravenna_available_for_test(&mut self, available: bool) {
        self.ravenna_available = available;
    }
}

fn format_system_endpoint(ep: &SystemEndpoint) -> String {
    if let Some(node) = &ep.node {
        format!("{node}:{}", ep.port)
    } else if let Some(alias) = &ep.alias {
        format!("{alias}:{}", ep.port)
    } else {
        ep.port.clone()
    }
}

/// Extract an uncompressed tar of a directory package into `dest`.
///
/// Rejects absolute paths, `..`, and entries that would escape `dest`.
pub fn extract_package_tar(archive: &[u8], dest: &Path) -> Result<(), ServiceError> {
    use std::io::Cursor;
    use tar::Archive;
    fs::create_dir_all(dest).map_err(|e| ServiceError::msg(e.to_string()))?;
    let mut archive = Archive::new(Cursor::new(archive));
    let entries = archive
        .entries()
        .map_err(|e| ServiceError::msg(e.to_string()))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| ServiceError::msg(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| ServiceError::msg(e.to_string()))?
            .into_owned();
        if path.is_absolute() {
            return Err(ServiceError::msg("package tar path is absolute"));
        }
        for c in path.components() {
            match c {
                Component::Normal(_) | Component::CurDir => {}
                _ => {
                    return Err(ServiceError::msg(
                        "package tar path escapes destination",
                    ));
                }
            }
        }
        let out = dest.join(&path);
        if !out.starts_with(dest) {
            return Err(ServiceError::msg(
                "package tar path escapes destination",
            ));
        }
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&out).map_err(|e| ServiceError::msg(e.to_string()))?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| ServiceError::msg(e.to_string()))?;
            }
            entry
                .unpack(&out)
                .map_err(|e| ServiceError::msg(e.to_string()))?;
        }
    }
    Ok(())
}

/// Build an uncompressed tar of a local directory package (for CLI/tests).
pub fn pack_package_tar(src: &Path) -> Result<Vec<u8>, ServiceError> {
    use std::io::Cursor;
    use tar::Builder;
    let mut buf = Cursor::new(Vec::new());
    {
        let mut builder = Builder::new(&mut buf);
        append_dir(&mut builder, src, Path::new(""))?;
        builder
            .finish()
            .map_err(|e| ServiceError::msg(e.to_string()))?;
    }
    Ok(buf.into_inner())
}

fn append_dir<W: io::Write>(
    builder: &mut tar::Builder<W>,
    src: &Path,
    prefix: &Path,
) -> Result<(), ServiceError> {
    for entry in fs::read_dir(src).map_err(|e| ServiceError::msg(e.to_string()))? {
        let entry = entry.map_err(|e| ServiceError::msg(e.to_string()))?;
        let name = entry.file_name();
        let rel = prefix.join(&name);
        let path = entry.path();
        let meta = entry
            .metadata()
            .map_err(|e| ServiceError::msg(e.to_string()))?;
        if meta.is_dir() {
            builder
                .append_dir(&rel, &path)
                .map_err(|e| ServiceError::msg(e.to_string()))?;
            append_dir(builder, &path, &rel)?;
        } else {
            let mut file = File::open(&path).map_err(|e| ServiceError::msg(e.to_string()))?;
            builder
                .append_file(&rel, &mut file)
                .map_err(|e| ServiceError::msg(e.to_string()))?;
        }
    }
    Ok(())
}

fn sanitize_rel_path(path: &str) -> Result<String, ServiceError> {
    if path.is_empty() || Path::new(path).is_absolute() {
        return Err(ServiceError::msg("invalid path"));
    }
    let p = Path::new(path);
    for c in p.components() {
        match c {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => return Err(ServiceError::msg("path escapes session work area")),
        }
    }
    Ok(path.to_string())
}

fn list_files_rec(
    root: &Path,
    dir: &Path,
    out: &mut Vec<SessionFileEntry>,
) -> Result<(), ServiceError> {
    for entry in fs::read_dir(dir).map_err(|e| ServiceError::msg(e.to_string()))? {
        let entry = entry.map_err(|e| ServiceError::msg(e.to_string()))?;
        let meta = entry
            .metadata()
            .map_err(|e| ServiceError::msg(e.to_string()))?;
        let rel = entry
            .path()
            .strip_prefix(root)
            .map_err(|e| ServiceError::msg(e.to_string()))?
            .to_string_lossy()
            .to_string();
        if meta.is_dir() {
            out.push(SessionFileEntry {
                path: rel,
                size: 0,
                directory: true,
            });
            list_files_rec(root, &entry.path(), out)?;
        } else {
            out.push(SessionFileEntry {
                path: rel,
                size: meta.len(),
                directory: false,
            });
        }
    }
    Ok(())
}

/// Helper to build a SystemLink from parts.
pub fn system_link(
    instance: &str,
    role: &str,
    node: Option<&str>,
    alias: Option<&str>,
    port: &str,
) -> SystemLink {
    SystemLink {
        from: RoleRefSerde {
            instance: instance.into(),
            role: role.into(),
        },
        to: SystemEndpoint {
            alias: alias.map(str::to_string),
            node: node.map(str::to_string),
            port: port.into(),
        },
    }
}
