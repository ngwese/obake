// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Resolve and stream Singularity instance stdout/stderr log files.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Log path resolution errors.
#[derive(Debug, Error)]
pub enum LogError {
    /// Underlying I/O or process failure.
    #[error("logs: {0}")]
    Message(String),
}

impl LogError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

/// Stdout and stderr paths for one Singularity instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFiles {
    /// `.out` path.
    pub out: PathBuf,
    /// `.err` path.
    pub err: PathBuf,
}

/// Resolves Singularity instance log file paths.
pub trait LogPaths: Send + Sync {
    /// Resolve `.out` and `.err` for a Singularity instance name.
    fn resolve(&self, singularity_name: &str) -> Result<LogFiles, LogError>;
}

/// Production resolver using `singularity instance list -j`.
#[derive(Debug, Default)]
pub struct SingularityLogPaths;

impl SingularityLogPaths {
    /// Create a resolver.
    pub fn new() -> Self {
        Self
    }
}

impl LogPaths for SingularityLogPaths {
    fn resolve(&self, singularity_name: &str) -> Result<LogFiles, LogError> {
        let output = Command::new("singularity")
            .args(["instance", "list", "-j"])
            .output()
            .map_err(|e| LogError::msg(e.to_string()))?;
        if !output.status.success() {
            return Err(LogError::msg(format!(
                "singularity instance list failed: {}",
                output.status
            )));
        }
        let v: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| LogError::msg(e.to_string()))?;
        let instances = v
            .get("instances")
            .and_then(|i| i.as_array())
            .ok_or_else(|| LogError::msg("singularity instance list: missing instances"))?;
        for inst in instances {
            let name = inst.get("instance").and_then(|n| n.as_str()).unwrap_or("");
            if name != singularity_name {
                continue;
            }
            let out = inst
                .get("logOutPath")
                .and_then(|p| p.as_str())
                .ok_or_else(|| LogError::msg("missing logOutPath"))?;
            let err = inst
                .get("logErrPath")
                .and_then(|p| p.as_str())
                .ok_or_else(|| LogError::msg("missing logErrPath"))?;
            return Ok(LogFiles {
                out: PathBuf::from(out),
                err: PathBuf::from(err),
            });
        }
        Err(LogError::msg(format!(
            "no singularity logs for instance `{singularity_name}`"
        )))
    }
}

/// In-memory map of singularity name → log files (tests).
#[derive(Debug, Default)]
pub struct MapLogPaths {
    inner: Mutex<HashMap<String, LogFiles>>,
}

impl MapLogPaths {
    /// Empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register paths for a Singularity instance name.
    pub fn insert(&self, singularity_name: impl Into<String>, files: LogFiles) {
        self.inner
            .lock()
            .unwrap()
            .insert(singularity_name.into(), files);
    }
}

impl LogPaths for MapLogPaths {
    fn resolve(&self, singularity_name: &str) -> Result<LogFiles, LogError> {
        self.inner
            .lock()
            .unwrap()
            .get(singularity_name)
            .cloned()
            .ok_or_else(|| {
                LogError::msg(format!(
                    "no singularity logs for instance `{singularity_name}`"
                ))
            })
    }
}

/// One instance's logs ready to stream.
#[derive(Debug, Clone)]
pub struct InstanceLogTarget {
    /// Rig instance name (prefix).
    pub name: String,
    /// Log files.
    pub files: LogFiles,
}

/// Emit prefixed lines from one or more instance log files.
///
/// Reads `.err` then `.out` for each instance (Chuck writes to stderr).
/// When `follow` is true, polls for appended bytes until `still_active`
/// returns false.
pub fn stream_instance_logs<F>(
    targets: &[InstanceLogTarget],
    follow: bool,
    mut still_active: F,
    mut emit: impl FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<()>
where
    F: FnMut() -> bool,
{
    let mut readers: Vec<(String, FileTail)> = Vec::new();
    for t in targets {
        readers.push((t.name.clone(), FileTail::open(&t.files.err)?));
        readers.push((t.name.clone(), FileTail::open(&t.files.out)?));
    }

    loop {
        let mut progressed = false;
        for (name, tail) in &mut readers {
            let chunk = tail.read_available()?;
            if chunk.is_empty() {
                continue;
            }
            progressed = true;
            emit_prefixed_chunk(name, &chunk, &mut tail.partial, &mut emit)?;
        }
        if !follow {
            for (name, tail) in &mut readers {
                if tail.partial.is_empty() {
                    continue;
                }
                let mut line = format!("[{name}] ").into_bytes();
                line.append(&mut tail.partial);
                if !line.ends_with(b"\n") {
                    line.push(b'\n');
                }
                emit(&line)?;
            }
            break;
        }
        if !still_active() {
            break;
        }
        if !progressed {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
    Ok(())
}

struct FileTail {
    file: Option<File>,
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
}

impl FileTail {
    fn open(path: &Path) -> io::Result<Self> {
        let file = match File::open(path) {
            Ok(f) => Some(f),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        Ok(Self {
            file,
            path: path.to_path_buf(),
            offset: 0,
            partial: Vec::new(),
        })
    }

    fn read_available(&mut self) -> io::Result<Vec<u8>> {
        if self.file.is_none() {
            match File::open(&self.path) {
                Ok(f) => self.file = Some(f),
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
                Err(e) => return Err(e),
            }
        }
        let file = self.file.as_mut().unwrap();
        file.seek(SeekFrom::Start(self.offset))?;
        let mut buf = Vec::new();
        let n = file.read_to_end(&mut buf)?;
        self.offset += n as u64;
        Ok(buf)
    }
}

/// Process a chunk for one named stream, updating `partial` and emitting
/// complete prefixed lines.
pub fn emit_prefixed_chunk(
    name: &str,
    chunk: &[u8],
    partial: &mut Vec<u8>,
    mut emit: impl FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<()> {
    partial.extend_from_slice(chunk);
    while let Some(pos) = partial.iter().position(|&b| b == b'\n') {
        let line_bytes: Vec<u8> = partial.drain(..=pos).collect();
        let mut out = format!("[{name}] ").into_bytes();
        out.extend_from_slice(&line_bytes);
        emit(&out)?;
    }
    Ok(())
}

/// Shared handle for tests that want to inject paths after construction.
pub type SharedLogPaths = Arc<dyn LogPaths>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn map_paths_resolve_and_miss() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("a.out");
        let err = dir.path().join("a.err");
        fs_write(&out, b"o\n");
        fs_write(&err, b"e\n");
        let map = MapLogPaths::new();
        map.insert(
            "obake-sid-tone",
            LogFiles {
                out: out.clone(),
                err: err.clone(),
            },
        );
        let got = map.resolve("obake-sid-tone").unwrap();
        assert_eq!(got.out, out);
        assert!(map.resolve("missing").is_err());
    }

    #[test]
    fn dump_prefixes_err_then_out() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("t.out");
        let err = dir.path().join("t.err");
        fs_write(&err, b"err1\nerr2\n");
        fs_write(&out, b"out1\n");
        let targets = [InstanceLogTarget {
            name: "tone".into(),
            files: LogFiles { out, err },
        }];
        let mut buf = Vec::new();
        stream_instance_logs(&targets, false, || true, |c| {
            buf.extend_from_slice(c);
            Ok(())
        })
        .unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("[tone] err1\n"));
        assert!(text.contains("[tone] err2\n"));
        assert!(text.contains("[tone] out1\n"));
        let err_pos = text.find("[tone] err1").unwrap();
        let out_pos = text.find("[tone] out1").unwrap();
        assert!(err_pos < out_pos);
    }

    #[test]
    fn follow_sees_appended_line() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("t.out");
        let err = dir.path().join("t.err");
        fs_write(&err, b"start\n");
        fs_write(&out, b"");
        let err_path = err.clone();
        let targets = [InstanceLogTarget {
            name: "tone".into(),
            files: LogFiles { out, err },
        }];
        let mut buf = Vec::new();
        let mut ticks = 0u32;
        stream_instance_logs(
            &targets,
            true,
            || {
                ticks += 1;
                if ticks == 2 {
                    let mut f = std::fs::OpenOptions::new()
                        .append(true)
                        .open(&err_path)
                        .unwrap();
                    f.write_all(b"later\n").unwrap();
                }
                ticks < 8
            },
            |c| {
                buf.extend_from_slice(c);
                Ok(())
            },
        )
        .unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("[tone] start\n"));
        assert!(text.contains("[tone] later\n"));
    }

    fn fs_write(path: &Path, bytes: &[u8]) {
        let mut f = File::create(path).unwrap();
        f.write_all(bytes).unwrap();
    }
}
