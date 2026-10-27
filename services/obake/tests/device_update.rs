// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! `obake device update` against fake ssh/scp on PATH.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use obake::device;
use tempfile::tempdir;

static PATH_LOCK: Mutex<()> = Mutex::new(());

fn write_exe(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o755)
        .open(&path)
        .unwrap();
    f.write_all(body.as_bytes()).unwrap();
}

fn with_path<T>(bin_dir: &Path, f: impl FnOnce() -> T) -> T {
    let _guard = PATH_LOCK.lock().unwrap();
    let old = std::env::var_os("PATH");
    std::env::set_var("PATH", bin_dir);
    let out = f();
    match old {
        Some(v) => std::env::set_var("PATH", v),
        None => std::env::remove_var("PATH"),
    }
    out
}

#[test]
fn update_probes_copies_and_applies_with_flags() {
    let root = tempdir().unwrap();
    let bin = root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let log = root.path().join("commands.log");
    let bundles = root.path().join("bundles");
    fs::create_dir(&bundles).unwrap();
    fs::write(bundles.join("obake-0.8.0-a.raucb"), b"bundle").unwrap();
    fs::write(bundles.join("obake-0.7.0-a.raucb"), b"old").unwrap();
    fs::write(bundles.join("obake-0.8.0-b.raucb"), b"other").unwrap();

    let log_path = log.to_str().unwrap();
    write_exe(
        &bin,
        "ssh",
        &format!(
            r#"#!/bin/sh
echo "ssh $*" >>"{log}"
# Drop optional -tt, then the user@host target.
while [ $# -gt 0 ]; do
  case "$1" in
    -tt|-t) shift ;;
    *) break ;;
  esac
done
shift
cmd="$*"
case "$cmd" in
  "obake-update status")
    echo "running version: 0.7.0"
    echo "active slot: rootfs.1 (B)"
    echo "pending/activated:"
    exit 0
    ;;
  "mkdir -p /home/.obake-update")
    exit 0
    ;;
  "obake-update apply --cleanup --reboot /home/.obake-update/obake-0.8.0-a.raucb")
    echo "obake-update: installing /home/.obake-update/obake-0.8.0-a.raucb (local)" >&2
    echo "obake-update: removed /home/.obake-update/obake-0.8.0-a.raucb" >&2
    echo "obake-update: rebooting" >&2
    exit 0
    ;;
  *)
    echo "unexpected ssh command: $cmd" >&2
    exit 2
    ;;
esac
"#,
            log = log_path
        ),
    );
    write_exe(
        &bin,
        "scp",
        &format!(
            r#"#!/bin/sh
echo "scp $*" >>"{log}"
case "$*" in
  *"obake@studio.local:/home/.obake-update/obake-0.8.0-a.raucb")
    exit 0
    ;;
  *)
    echo "unexpected scp: $*" >&2
    exit 2
    ;;
esac
"#,
            log = log_path
        ),
    );

    with_path(&bin, || {
        device::update("obake@studio.local", &bundles, false).unwrap();
    });

    let logged = fs::read_to_string(&log).unwrap();
    assert!(logged.contains("obake-update status"), "{logged}");
    assert!(logged.contains("scp "), "{logged}");
    assert!(
        logged.contains(
            "obake-update apply --cleanup --reboot /home/.obake-update/obake-0.8.0-a.raucb"
        ),
        "{logged}"
    );
}

#[test]
fn cli_requires_target() {
    let out = Command::new(env!("CARGO_BIN_EXE_obake"))
        .args(["device", "update", "./bundles"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--target") || err.contains("required"),
        "stderr={err}"
    );
}
