// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! `obake device update`: SSH-backed OS bundle push and apply.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Device-update errors.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct DeviceError(String);

impl DeviceError {
    fn msg(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Boot slot letter in a bundle filename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Slot A / rootfs.0.
    A,
    /// Slot B / rootfs.1.
    B,
}

impl Slot {
    /// Parse `a` or `b`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "a" | "A" => Some(Self::A),
            "b" | "B" => Some(Self::B),
            _ => None,
        }
    }

    /// Filename letter.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
        }
    }

    /// The other A/B slot (inactive when `self` is active).
    pub fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

/// Strict `MAJOR.MINOR.PATCH` image version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemVer {
    /// Major.
    pub major: u64,
    /// Minor.
    pub minor: u64,
    /// Patch.
    pub patch: u64,
}

impl SemVer {
    /// Parse `MAJOR.MINOR.PATCH`.
    pub fn parse(s: &str) -> Result<Self, DeviceError> {
        let mut parts = s.split('.');
        let major = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| DeviceError::msg(format!("invalid semver: {s}")))?;
        let minor = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| DeviceError::msg(format!("invalid semver: {s}")))?;
        let patch = parts
            .next()
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| DeviceError::msg(format!("invalid semver: {s}")))?;
        if parts.next().is_some() {
            return Err(DeviceError::msg(format!("invalid semver: {s}")));
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    /// True when `self >= other`.
    pub fn ge(self, other: Self) -> bool {
        self.cmp(&other) != Ordering::Less
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Parsed `obake-<semver>-<slot>.raucb` basename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleName {
    /// Bundle version.
    pub version: SemVer,
    /// Target slot letter.
    pub slot: Slot,
}

impl BundleName {
    /// Parse a basename (not a path).
    pub fn parse(name: &str) -> Result<Self, DeviceError> {
        let rest = name
            .strip_prefix("obake-")
            .and_then(|s| s.strip_suffix(".raucb"))
            .ok_or_else(|| DeviceError::msg(format!("not an obake bundle name: {name}")))?;
        let (ver, slot) = rest
            .rsplit_once('-')
            .ok_or_else(|| DeviceError::msg(format!("not an obake bundle name: {name}")))?;
        let version = SemVer::parse(ver)?;
        let slot = Slot::parse(slot)
            .ok_or_else(|| DeviceError::msg(format!("not an obake bundle name: {name}")))?;
        Ok(Self { version, slot })
    }
}

/// Running version and active slot from `obake-update status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceStatus {
    /// `/etc/obake/image-version`.
    pub version: SemVer,
    /// Currently booted slot.
    pub active: Slot,
}

impl DeviceStatus {
    /// Inactive slot that a bundle must target.
    pub fn inactive(&self) -> Slot {
        self.active.other()
    }
}

/// Parse `obake-update status` stdout.
pub fn parse_status(text: &str) -> Result<DeviceStatus, DeviceError> {
    let mut version = None;
    let mut active = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("running version:") {
            version = Some(SemVer::parse(v.trim())?);
        } else if let Some(rest) = line.strip_prefix("active slot:") {
            active = Some(parse_active_slot(rest.trim())?);
        }
    }
    let version = version.ok_or_else(|| DeviceError::msg("status missing running version"))?;
    let active = active.ok_or_else(|| DeviceError::msg("status missing active slot"))?;
    Ok(DeviceStatus { version, active })
}

fn parse_active_slot(s: &str) -> Result<Slot, DeviceError> {
    // Examples: "rootfs.0", "rootfs.1 (B)", "rootfs.0 (A)"
    if s.contains('B') || s.contains("rootfs.1") {
        return Ok(Slot::B);
    }
    if s.contains('A') || s.contains("rootfs.0") {
        return Ok(Slot::A);
    }
    Err(DeviceError::msg(format!("cannot parse active slot: {s}")))
}

/// Choose a local bundle path for update.
pub fn resolve_bundle(
    path: &Path,
    status: &DeviceStatus,
    force: bool,
) -> Result<PathBuf, DeviceError> {
    if path.is_dir() {
        if force {
            return Err(DeviceError::msg(
                "--force is only valid with a specific bundle file",
            ));
        }
        select_from_dir(path, status.inactive(), status.version)
    } else if path.is_file() {
        validate_file(path, status.inactive(), status.version, force)
    } else {
        Err(DeviceError::msg(format!(
            "path not found: {}",
            path.display()
        )))
    }
}

/// Directory: latest eligible `obake-<semver>-<inactive>.raucb` with version ≥ min.
pub fn select_from_dir(
    dir: &Path,
    inactive: Slot,
    min_version: SemVer,
) -> Result<PathBuf, DeviceError> {
    let mut best: Option<(SemVer, PathBuf)> = None;
    let entries = fs::read_dir(dir).map_err(|e| DeviceError::msg(format!("read dir: {e}")))?;
    for entry in entries {
        let entry = entry.map_err(|e| DeviceError::msg(format!("read dir: {e}")))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Ok(bundle) = BundleName::parse(name) else {
            continue;
        };
        if bundle.slot != inactive {
            continue;
        }
        if !bundle.version.ge(min_version) {
            continue;
        }
        match &best {
            Some((v, _)) if bundle.version <= *v => {}
            _ => best = Some((bundle.version, path)),
        }
    }
    best.map(|(_, p)| p).ok_or_else(|| {
        DeviceError::msg(format!(
            "no obake-*-{}.raucb with version >= {min_version} in {}",
            inactive.as_str(),
            dir.display()
        ))
    })
}

/// File: enforce slot and semver unless `force`.
pub fn validate_file(
    path: &Path,
    inactive: Slot,
    min_version: SemVer,
    force: bool,
) -> Result<PathBuf, DeviceError> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| DeviceError::msg(format!("invalid bundle path: {}", path.display())))?;
    if force {
        return Ok(path.to_path_buf());
    }
    let bundle = BundleName::parse(name)?;
    if bundle.slot != inactive {
        return Err(DeviceError::msg(format!(
            "bundle slot {} does not match inactive slot {}",
            bundle.slot.as_str(),
            inactive.as_str()
        )));
    }
    if !bundle.version.ge(min_version) {
        return Err(DeviceError::msg(format!(
            "bundle version {} is older than device version {min_version}",
            bundle.version
        )));
    }
    Ok(path.to_path_buf())
}

/// `obake device update --target user@host PATH [--force]`.
pub fn update(target: &str, path: &Path, force: bool) -> Result<(), DeviceError> {
    validate_ssh_target(target)?;
    eprintln!("obake: probing {target}");
    let status_text = ssh_capture(target, "obake-update status")?;
    let status = parse_status(&status_text)?;
    eprintln!(
        "obake: device version {} active slot {} (inactive {})",
        status.version,
        status.active.as_str(),
        status.inactive().as_str()
    );
    let local = resolve_bundle(path, &status, force)?;
    let basename = local
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| DeviceError::msg("bundle basename"))?;
    eprintln!("obake: selected {}", local.display());
    // Stage under the SSH user's home — `/home/.obake-update` is only
    // writable by root (HTTPS fetch path inside obake-update).
    let stage_dir = remote_stage_dir(target)?;
    let remote = format!("{stage_dir}/{basename}");
    eprintln!("obake: copying to {target}:{remote}");
    scp_to(target, &local, &remote)?;
    let apply = format!("obake-update apply --cleanup --reboot {remote}");
    eprintln!("obake: running {apply}");
    // Allocate a TTY so remote stderr (install/cleanup/reboot lines) streams
    // live instead of buffering until the session ends on reboot.
    match ssh_run_tty(target, &apply) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Reboot often closes the SSH session after a successful apply.
            let msg = e.to_string();
            if msg.contains("ssh exited") {
                eprintln!("obake: remote session ended after apply (reboot expected)");
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

/// Create `$HOME/.obake-update` on the target and return its absolute path.
fn remote_stage_dir(target: &str) -> Result<String, DeviceError> {
    let out = ssh_capture(
        target,
        "mkdir -p \"$HOME/.obake-update\" && printf %s \"$HOME/.obake-update\"",
    )?;
    let dir = out.trim();
    if dir.is_empty() || !dir.starts_with('/') {
        return Err(DeviceError::msg(format!(
            "could not resolve remote stage dir: {out:?}"
        )));
    }
    Ok(dir.to_string())
}

fn validate_ssh_target(target: &str) -> Result<(), DeviceError> {
    target
        .split_once('@')
        .filter(|(u, h)| !u.is_empty() && !h.is_empty())
        .map(|_| ())
        .ok_or_else(|| DeviceError::msg(format!("expected user@host, got {target}")))
}

fn ssh_capture(target: &str, remote: &str) -> Result<String, DeviceError> {
    let output = Command::new("ssh")
        .arg(target)
        .arg(remote)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| DeviceError::msg(format!("ssh: {e}")))?;
    if !output.status.success() {
        return Err(DeviceError::msg(format!("ssh exited {}", output.status)));
    }
    String::from_utf8(output.stdout).map_err(|e| DeviceError::msg(format!("ssh utf8: {e}")))
}

fn ssh_run_tty(target: &str, remote: &str) -> Result<(), DeviceError> {
    // -tt forces a remote TTY even without a local one (CI / pipes).
    ssh_status(
        Command::new("ssh")
            .arg("-tt")
            .arg(target)
            .arg(remote),
    )
}

fn ssh_status(cmd: &mut Command) -> Result<(), DeviceError> {
    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| DeviceError::msg(format!("ssh: {e}")))?;
    if !status.success() {
        return Err(DeviceError::msg(format!("ssh exited {status}")));
    }
    Ok(())
}

fn scp_to(target: &str, local: &Path, remote: &str) -> Result<(), DeviceError> {
    let dest = format!("{target}:{remote}");
    let status = Command::new("scp")
        .arg(local)
        .arg(&dest)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| DeviceError::msg(format!("scp: {e}")))?;
    if !status.success() {
        return Err(DeviceError::msg(format!("scp exited {status}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn semver_parse_and_order() {
        assert!(SemVer::parse("0.7.0").unwrap().ge(SemVer::parse("0.7.0").unwrap()));
        assert!(SemVer::parse("0.8.0").unwrap().ge(SemVer::parse("0.7.0").unwrap()));
        assert!(!SemVer::parse("0.7.0").unwrap().ge(SemVer::parse("0.8.0").unwrap()));
        assert!(SemVer::parse("1.0.0").unwrap() > SemVer::parse("0.9.9").unwrap());
        assert!(SemVer::parse("0.7").is_err());
        assert!(SemVer::parse("0.7.0.1").is_err());
        assert!(SemVer::parse("v0.7.0").is_err());
    }

    #[test]
    fn bundle_name_parse() {
        let b = BundleName::parse("obake-0.8.0-a.raucb").unwrap();
        assert_eq!(b.version, SemVer::parse("0.8.0").unwrap());
        assert_eq!(b.slot, Slot::A);
        assert!(BundleName::parse("obake-0.8.0.raucb").is_err());
        assert!(BundleName::parse("other-0.8.0-a.raucb").is_err());
        assert!(BundleName::parse("obake-0.8.0-c.raucb").is_err());
    }

    #[test]
    fn parse_status_active_markers() {
        let s = parse_status(
            "running version: 0.7.0\nactive slot: rootfs.1 (B)\npending/activated: \n",
        )
        .unwrap();
        assert_eq!(s.version, SemVer::parse("0.7.0").unwrap());
        assert_eq!(s.active, Slot::B);
        assert_eq!(s.inactive(), Slot::A);

        let s = parse_status("running version: 1.2.3\nactive slot: rootfs.0\n").unwrap();
        assert_eq!(s.active, Slot::A);
        assert_eq!(s.inactive(), Slot::B);
    }

    #[test]
    fn select_directory_newest_eligible() {
        let dir = tempdir().unwrap();
        for name in [
            "obake-0.7.0-a.raucb",
            "obake-0.8.0-a.raucb",
            "obake-0.8.0-b.raucb",
        ] {
            fs::write(dir.path().join(name), b"x").unwrap();
        }
        let picked = select_from_dir(dir.path(), Slot::A, SemVer::parse("0.7.0").unwrap()).unwrap();
        assert_eq!(
            picked.file_name().unwrap().to_str().unwrap(),
            "obake-0.8.0-a.raucb"
        );
    }

    #[test]
    fn select_directory_nothing_new_enough() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("obake-0.7.0-a.raucb"), b"x").unwrap();
        let err = select_from_dir(dir.path(), Slot::A, SemVer::parse("0.8.0").unwrap()).unwrap_err();
        assert!(err.to_string().contains("no obake"));
    }

    #[test]
    fn select_directory_same_version() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("obake-0.7.0-b.raucb"), b"x").unwrap();
        let picked = select_from_dir(dir.path(), Slot::B, SemVer::parse("0.7.0").unwrap()).unwrap();
        assert!(picked.ends_with("obake-0.7.0-b.raucb"));
    }

    #[test]
    fn validate_file_rejects_wrong_slot_and_older() {
        let dir = tempdir().unwrap();
        let wrong = dir.path().join("obake-0.8.0-b.raucb");
        fs::write(&wrong, b"x").unwrap();
        let err = validate_file(&wrong, Slot::A, SemVer::parse("0.7.0").unwrap(), false).unwrap_err();
        assert!(err.to_string().contains("slot"));

        let older = dir.path().join("obake-0.7.0-a.raucb");
        fs::write(&older, b"x").unwrap();
        let err = validate_file(&older, Slot::A, SemVer::parse("0.8.0").unwrap(), false).unwrap_err();
        assert!(err.to_string().contains("older"));
    }

    #[test]
    fn force_bypasses_checks_file_only() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("obake-0.7.0-b.raucb");
        fs::write(&file, b"x").unwrap();
        let status = DeviceStatus {
            version: SemVer::parse("0.8.0").unwrap(),
            active: Slot::B, // inactive = A
        };
        let picked = resolve_bundle(&file, &status, true).unwrap();
        assert_eq!(picked, file);
        let err = resolve_bundle(dir.path(), &status, true).unwrap_err();
        assert!(err.to_string().contains("--force"));
    }
}
