// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! `obake auth login` against a fake ssh on PATH.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::sync::Mutex;

use obake::auth::login;
use tempfile::tempdir;

static PATH_LOCK: Mutex<()> = Mutex::new(());

fn write_ssh(dir: &Path, body: &str) {
    let path = dir.join("ssh");
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o755)
        .open(&path)
        .unwrap();
    f.write_all(body.as_bytes()).unwrap();
}

fn with_ssh<T>(body: &str, f: impl FnOnce(&Path) -> T) -> T {
    let _guard = PATH_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    write_ssh(dir.path(), body);
    let old = std::env::var_os("PATH");
    std::env::set_var("PATH", dir.path());
    let out = f(dir.path());
    match old {
        Some(v) => std::env::set_var("PATH", v),
        None => std::env::remove_var("PATH"),
    }
    out
}

const OK_SSH: &str = r#"#!/bin/sh
case "$*" in
  *'PATH="$HOME/.local/bin:$PATH" obake auth credential'*) ;;
  *) echo "unexpected remote command: $*" >&2; exit 2 ;;
esac
printf '%s\n' '{"token":"tok-studio","port":9477,"fingerprint":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}'
"#;

#[test]
fn login_writes_host_and_failed_ssh_keeps_file() {
    let cfg_dir = tempdir().unwrap();
    let cfg = cfg_dir.path().join("config.yaml");
    fs::write(
        &cfg,
        r#"
default: spare.local
hosts:
  spare.local:
    endpoint: https://spare.local:9477
    token: tok-spare
    fingerprint: sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
"#,
    )
    .unwrap();
    let before = fs::read(&cfg).unwrap();

    with_ssh("#!/bin/sh\nexit 1\n", |_| {
        let err = login(&cfg, "obake@studio.local").unwrap_err();
        assert!(err.to_string().contains("ssh"));
    });
    assert_eq!(fs::read(&cfg).unwrap(), before);

    with_ssh(OK_SSH, |_| {
        login(&cfg, "obake@studio.local").unwrap();
    });
    let mode = fs::metadata(&cfg).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    let loaded = obake::load_config(&cfg).unwrap();
    assert_eq!(loaded.default.as_deref(), Some("spare.local"));
    let studio = &loaded.hosts["studio.local"];
    assert_eq!(studio.endpoint, "https://studio.local:9477");
    assert_eq!(studio.token, "tok-studio");
    assert!(studio.fingerprint.as_deref().unwrap().starts_with("sha256:"));
    assert_eq!(loaded.hosts["spare.local"].token, "tok-spare");

    with_ssh(
        r#"#!/bin/sh
case "$*" in
  *'PATH="$HOME/.local/bin:$PATH" obake auth credential'*) ;;
  *) echo "unexpected remote command: $*" >&2; exit 2 ;;
esac
printf '%s\n' '{"token":"tok-studio-2","port":9477,"fingerprint":"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}'
"#,
        |_| login(&cfg, "obake@studio.local").unwrap(),
    );
    let loaded = obake::load_config(&cfg).unwrap();
    assert_eq!(loaded.hosts["studio.local"].token, "tok-studio-2");
    assert_eq!(loaded.hosts["spare.local"].token, "tok-spare");
    assert_eq!(
        loaded.hosts["spare.local"].fingerprint.as_deref(),
        Some("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    );
}

#[test]
fn first_login_sets_default() {
    let cfg_dir = tempdir().unwrap();
    let cfg = cfg_dir.path().join("config.yaml");
    with_ssh(OK_SSH, |_| login(&cfg, "obake@studio.local").unwrap());
    let loaded = obake::load_config(&cfg).unwrap();
    assert_eq!(loaded.default.as_deref(), Some("studio.local"));
}
