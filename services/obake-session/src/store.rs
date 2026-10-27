// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Durable session snapshots with fsync-before-ack.

use crate::config::RoleRef;
use crate::paths::Paths;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use thiserror::Error;

/// Persistence errors.
#[derive(Debug, Error)]
pub enum StoreError {
    /// I/O failure.
    #[error("session store i/o: {0}")]
    Io(#[from] io::Error),
    /// YAML failure.
    #[error("session store yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Session start mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionMode {
    /// Empty work area, no system links.
    Fresh,
    /// Copy work area and system links from another session.
    Fork,
}

/// System link endpoint: alias+port or node+port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemEndpoint {
    /// Host-profile card alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// PipeWire node name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// Port name on that node / alias.
    pub port: String,
}

/// System link from an instance role to a host endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemLink {
    /// Instance role.
    pub from: RoleRefSerde,
    /// Host endpoint.
    pub to: SystemEndpoint,
}

/// Serializable role ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleRefSerde {
    /// Instance key.
    pub instance: String,
    /// Role name.
    pub role: String,
}

impl From<&RoleRef> for RoleRefSerde {
    fn from(r: &RoleRef) -> Self {
        Self {
            instance: r.instance.clone(),
            role: r.role.clone(),
        }
    }
}

impl From<&RoleRefSerde> for RoleRef {
    fn from(r: &RoleRefSerde) -> Self {
        Self {
            instance: r.instance.clone(),
            role: r.role.clone(),
        }
    }
}

/// Durable session record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Session id.
    pub id: String,
    /// Rig name.
    pub rig: String,
    /// Fresh or fork.
    pub mode: SessionMode,
    /// Source session when forked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,
    /// System links owned by this session.
    #[serde(default)]
    pub system_links: Vec<SystemLink>,
}

/// Write `path` atomically: temp sibling, fsync file, rename, fsync dir.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    atomic_write_from_reader(path, &mut io::Cursor::new(bytes))
}

/// Stream `reader` into `path` atomically (temp sibling, fsync, rename).
///
/// On any error after creating the temp file, the temp is removed and the
/// previous contents of `path` (if any) are left unchanged.
pub fn atomic_write_from_reader(path: &Path, reader: &mut dyn Read) -> Result<(), StoreError> {
    let parent = path.parent().ok_or_else(|| {
        StoreError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path has no parent",
        ))
    })?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("write")
    ));
    let result = (|| -> Result<(), StoreError> {
        {
            let mut f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o644)
                .open(&tmp)?;
            io::copy(reader, &mut f)?;
            f.sync_all()?;
        }
        fs::rename(&tmp, path)?;
        let dir = File::open(parent)?;
        dir.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Persist a session snapshot.
pub fn write_session(paths: &Paths, record: &SessionRecord) -> Result<(), StoreError> {
    let yaml = serde_yaml::to_string(record)?;
    atomic_write(&paths.session_file(&record.id), yaml.as_bytes())
}

/// Load a session snapshot.
pub fn read_session(paths: &Paths, id: &str) -> Result<SessionRecord, StoreError> {
    let text = fs::read_to_string(paths.session_file(id))?;
    Ok(serde_yaml::from_str(&text)?)
}

/// List all recorded sessions.
pub fn list_sessions(paths: &Paths) -> Result<Vec<SessionRecord>, StoreError> {
    let dir = paths.state_dir.join("sessions");
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut entries: Vec<_> = fs::read_dir(&dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        out.push(serde_yaml::from_str(&text)?);
    }
    Ok(out)
}

/// Delete a session snapshot file.
pub fn delete_session_file(paths: &Paths, id: &str) -> Result<(), StoreError> {
    let path = paths.session_file(id);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Write the `current` marker (empty string means idle).
pub fn write_current(paths: &Paths, id: Option<&str>) -> Result<(), StoreError> {
    let content = id.unwrap_or("");
    atomic_write(&paths.current_file(), content.as_bytes())
}

/// Read the `current` marker.
pub fn read_current(paths: &Paths) -> Result<Option<String>, StoreError> {
    match fs::read_to_string(paths.current_file()) {
        Ok(s) => {
            let s = s.trim();
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(s.to_string()))
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Ensure the bearer token file exists (mode 0600); return its contents.
pub fn ensure_token(paths: &Paths) -> Result<String, StoreError> {
    let path = paths.token_file();
    match fs::read_to_string(&path) {
        Ok(s) => Ok(s.trim().to_string()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(paths.config_dir.as_path())?;
            let token = uuid::Uuid::new_v4().to_string();
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)?;
            f.write_all(token.as_bytes())?;
            f.sync_all()?;
            Ok(token)
        }
        Err(e) => Err(e.into()),
    }
}

/// Recursively copy a directory tree.
pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fsync_write_survives_reload() {
        let dir = tempdir().unwrap();
        let paths = Paths::under(dir.path());
        paths.ensure().unwrap();
        let record = SessionRecord {
            id: "abc".into(),
            rig: "quartet".into(),
            mode: SessionMode::Fresh,
            forked_from: None,
            system_links: vec![],
        };
        write_session(&paths, &record).unwrap();
        write_current(&paths, Some("abc")).unwrap();
        let loaded = read_session(&paths, "abc").unwrap();
        assert_eq!(loaded.rig, "quartet");
        assert!(loaded.system_links.is_empty());
        assert_eq!(read_current(&paths).unwrap().as_deref(), Some("abc"));
    }
}
