// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Local rig package scaffolding and packing.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Local scaffold / pack errors.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct RigLocalError(String);

impl RigLocalError {
    fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Scaffold `<name>/rig.yaml` and `<name>/work/` under `parent`.
///
/// Fails when the destination already exists.
pub fn init_package(parent: &Path, name: &str) -> Result<PathBuf, RigLocalError> {
    if name.is_empty() || name.contains('/') || name.contains(':') {
        return Err(RigLocalError::msg("invalid rig name"));
    }
    let root = parent.join(name);
    if root.exists() {
        return Err(RigLocalError::msg(format!(
            "{} already exists",
            root.display()
        )));
    }
    fs::create_dir_all(root.join("work")).map_err(|e| RigLocalError::msg(e.to_string()))?;
    let def = root.join("rig.yaml");
    let mut f = File::create(&def).map_err(|e| RigLocalError::msg(e.to_string()))?;
    writeln!(
        f,
        "# SPDX-FileCopyrightText: Copyright 2026 Greg Wuller\n\
         # SPDX-License-Identifier: MIT\n\
         #\n\
         # Rig `{name}`\n\
         \n\
         instances: {{}}\n\
         links: []\n"
    )
    .map_err(|e| RigLocalError::msg(e.to_string()))?;
    Ok(root)
}

/// Pack a directory package into an uncompressed tar (for install).
pub fn pack_directory(src: &Path) -> Result<Vec<u8>, RigLocalError> {
    if !src.join("rig.yaml").is_file() {
        return Err(RigLocalError::msg("directory package missing rig.yaml"));
    }
    let mut buf = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut buf);
        append_dir(&mut builder, src, Path::new(""))?;
        builder
            .finish()
            .map_err(|e| RigLocalError::msg(e.to_string()))?;
    }
    Ok(buf)
}

fn append_dir<W: Write>(
    builder: &mut tar::Builder<W>,
    src: &Path,
    prefix: &Path,
) -> Result<(), RigLocalError> {
    for entry in fs::read_dir(src).map_err(|e| RigLocalError::msg(e.to_string()))? {
        let entry = entry.map_err(|e| RigLocalError::msg(e.to_string()))?;
        let name = entry.file_name();
        let rel = prefix.join(&name);
        let path = entry.path();
        let meta = entry
            .metadata()
            .map_err(|e| RigLocalError::msg(e.to_string()))?;
        if meta.is_dir() {
            builder
                .append_dir(&rel, &path)
                .map_err(|e| RigLocalError::msg(e.to_string()))?;
            append_dir(builder, &path, &rel)?;
        } else {
            let mut file = File::open(&path).map_err(|e| RigLocalError::msg(e.to_string()))?;
            builder
                .append_file(&rel, &mut file)
                .map_err(|e| RigLocalError::msg(e.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_creates_and_refuses_existing() {
        let dir = tempdir().unwrap();
        let root = init_package(dir.path(), "tone").unwrap();
        assert!(root.join("rig.yaml").is_file());
        assert!(root.join("work").is_dir());
        let err = init_package(dir.path(), "tone").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }
}
