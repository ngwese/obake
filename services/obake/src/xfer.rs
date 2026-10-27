// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Local/remote path expansion for multi-file `file get` / `file put`.

use std::path::{Path, PathBuf};

/// One local file to upload, with the remote-relative name to use under a
/// directory destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSource {
    pub path: PathBuf,
    pub remote_name: String,
}

pub fn split_src_dest(args: &[String]) -> Result<(Vec<String>, String), String> {
    if args.len() < 2 {
        return Err("need at least one source and a destination".into());
    }
    let dest = args.last().cloned().unwrap();
    let sources = args[..args.len() - 1].to_vec();
    Ok((sources, dest))
}

pub fn is_dir_dest(dest: &str) -> bool {
    dest == "." || dest.ends_with('/') || Path::new(dest).is_dir()
}

pub fn expand_local_sources(patterns: &[String]) -> Result<Vec<LocalSource>, String> {
    let mut out = Vec::new();
    for pat in patterns {
        let path = Path::new(pat);
        if path.is_dir() {
            collect_local_files(path, path, &mut out)?;
            continue;
        }
        let mut matched = false;
        for entry in glob::glob(pat).map_err(|e| e.to_string())? {
            let p = entry.map_err(|e| e.to_string())?;
            if p.is_dir() {
                collect_local_files(&p, &p, &mut out)?;
            } else if p.is_file() {
                let name = file_name_str(&p)?;
                out.push(LocalSource {
                    path: p,
                    remote_name: name,
                });
            }
            matched = true;
        }
        if !matched {
            if path.is_file() {
                out.push(LocalSource {
                    path: path.to_path_buf(),
                    remote_name: file_name_str(path)?,
                });
            } else {
                return Err(format!("no local files matched: {pat}"));
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.dedup_by(|a, b| a.path == b.path);
    Ok(out)
}

fn collect_local_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<LocalSource>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        if p.is_dir() {
            collect_local_files(root, &p, out)?;
        } else if p.is_file() {
            let rel = p.strip_prefix(root).map_err(|e| e.to_string())?;
            let remote_name = rel
                .iter()
                .map(|c| c.to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.push(LocalSource {
                path: p,
                remote_name,
            });
        }
    }
    Ok(())
}

fn file_name_str(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "local path has no file name".into())
}

pub fn remote_target_for_put(
    source: &LocalSource,
    dest: &str,
    multi_or_dir: bool,
) -> String {
    if !multi_or_dir {
        return dest.trim_end_matches('/').to_string();
    }
    if dest == "." {
        return source.remote_name.clone();
    }
    let dest = dest.trim_end_matches('/');
    format!("{dest}/{}", source.remote_name)
}

/// Match remote work-area paths from `sessionFiles` entries.
///
/// Each entry is `(path, is_directory)`. Patterns may be globs, directory
/// prefixes (with or without a trailing `/`), `.` (all files), or exact paths.
pub fn match_remote_sources(
    entries: &[(String, bool)],
    patterns: &[String],
) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for pat in patterns {
        let mut matched = false;

        if pat == "." {
            for (path, is_dir) in entries {
                if !*is_dir {
                    out.push(path.clone());
                    matched = true;
                }
            }
            if !matched {
                return Err("no remote files matched: .".into());
            }
            continue;
        }

        if let Ok(pattern) = glob::Pattern::new(pat) {
            let opts = glob::MatchOptions {
                case_sensitive: true,
                require_literal_separator: true,
                require_literal_leading_dot: false,
            };
            for (path, is_dir) in entries {
                if !is_dir && pattern.matches_with(path, opts) {
                    out.push(path.clone());
                    matched = true;
                }
            }
        }

        let prefix = pat.trim_end_matches('/');
        let is_dir_ref = pat.ends_with('/')
            || entries.iter().any(|(p, d)| *d && p == prefix)
            || entries
                .iter()
                .any(|(p, d)| !*d && p.starts_with(&format!("{prefix}/")));
        if is_dir_ref {
            let pre = format!("{prefix}/");
            for (path, is_dir) in entries {
                if !*is_dir && path.starts_with(&pre) {
                    out.push(path.clone());
                    matched = true;
                }
            }
        }

        if !matched {
            if let Some((path, is_dir)) = entries.iter().find(|(p, _)| p == pat || p == prefix) {
                if !*is_dir {
                    out.push(path.clone());
                    matched = true;
                }
            }
        }
        if !matched {
            return Err(format!("no remote files matched: {pat}"));
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Whether `dest` should be treated as a remote directory for put (including
/// when it already exists as a directory in the work area).
pub fn is_remote_dir_dest(dest: &str, entries: &[(String, bool)]) -> bool {
    if is_dir_dest(dest) {
        return true;
    }
    let dest = dest.trim_end_matches('/');
    if dest == "." {
        return true;
    }
    entries.iter().any(|(p, d)| *d && p == dest)
        || entries
            .iter()
            .any(|(p, d)| !*d && p.starts_with(&format!("{dest}/")))
}

pub fn local_target_for_get(
    remote: &str,
    dest: &str,
    multi_or_dir: bool,
) -> Result<String, String> {
    if dest == "-" {
        return Ok("-".into());
    }
    let name = Path::new(remote)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "remote path has no file name".to_string())?;
    if !multi_or_dir {
        if dest == "." {
            return Ok(name.to_string());
        }
        return Ok(dest.to_string());
    }
    if dest == "." {
        return Ok(remote.to_string());
    }
    let dest = dest.trim_end_matches('/');
    Ok(format!("{dest}/{remote}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn split_requires_dest() {
        assert!(split_src_dest(&["a".into()]).is_err());
        let (srcs, dest) = split_src_dest(&["a".into(), "b".into(), "d".into()]).unwrap();
        assert_eq!(srcs, vec!["a", "b"]);
        assert_eq!(dest, "d");
    }

    #[test]
    fn put_targets() {
        let src = LocalSource {
            path: PathBuf::from("x.ck"),
            remote_name: "x.ck".into(),
        };
        assert_eq!(remote_target_for_put(&src, "renamed.ck", false), "renamed.ck");
        assert_eq!(remote_target_for_put(&src, ".", true), "x.ck");
        assert_eq!(remote_target_for_put(&src, "patches/", true), "patches/x.ck");
        assert_eq!(remote_target_for_put(&src, "patches", true), "patches/x.ck");
    }

    #[test]
    fn get_targets() {
        assert_eq!(
            local_target_for_get("a/b.wav", "out.wav", false).unwrap(),
            "out.wav"
        );
        assert_eq!(
            local_target_for_get("a/b.wav", ".", false).unwrap(),
            "b.wav"
        );
        assert_eq!(
            local_target_for_get("a/b.wav", ".", true).unwrap(),
            "a/b.wav"
        );
        assert_eq!(
            local_target_for_get("a/b.wav", "dl/", true).unwrap(),
            "dl/a/b.wav"
        );
    }

    #[test]
    fn match_globs_and_dirs() {
        let entries = vec![
            ("a.ck".into(), false),
            ("b.ck".into(), false),
            ("patches".into(), true),
            ("patches/x.ck".into(), false),
            ("patches/y.ck".into(), false),
        ];
        assert_eq!(
            match_remote_sources(&entries, &["*.ck".into()]).unwrap(),
            vec!["a.ck", "b.ck"]
        );
        assert_eq!(
            match_remote_sources(&entries, &["**/*.ck".into()]).unwrap(),
            vec!["a.ck", "b.ck", "patches/x.ck", "patches/y.ck"]
        );
        assert_eq!(
            match_remote_sources(&entries, &["patches/*.ck".into()]).unwrap(),
            vec!["patches/x.ck", "patches/y.ck"]
        );
        assert_eq!(
            match_remote_sources(&entries, &["patches".into()]).unwrap(),
            vec!["patches/x.ck", "patches/y.ck"]
        );
        assert_eq!(
            match_remote_sources(&entries, &["patches/".into()]).unwrap(),
            vec!["patches/x.ck", "patches/y.ck"]
        );
        assert_eq!(
            match_remote_sources(&entries, &[".".into()])
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            match_remote_sources(&entries, &["a.ck".into()]).unwrap(),
            vec!["a.ck"]
        );
    }

    #[test]
    fn expand_glob_and_dir() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.ck");
        let b = dir.path().join("sub");
        fs::create_dir_all(&b).unwrap();
        let c = b.join("c.ck");
        fs::write(&a, b"1").unwrap();
        fs::write(&c, b"2").unwrap();

        let pattern = dir.path().join("*.ck").display().to_string();
        let got = expand_local_sources(&[pattern]).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].remote_name, "a.ck");

        let tree = expand_local_sources(&[b.display().to_string()]).unwrap();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].remote_name, "c.ck");

        let nested = expand_local_sources(&[dir.path().display().to_string()]).unwrap();
        assert_eq!(nested.len(), 2);
        let names: Vec<_> = nested.iter().map(|s| s.remote_name.as_str()).collect();
        assert!(names.contains(&"a.ck"));
        assert!(names.contains(&"sub/c.ck"));
    }

    #[test]
    fn remote_dir_dest_detection() {
        let entries = vec![
            ("patches".into(), true),
            ("patches/x.ck".into(), false),
        ];
        assert!(is_remote_dir_dest("patches", &entries));
        assert!(is_remote_dir_dest("patches/", &entries));
        assert!(is_remote_dir_dest(".", &entries));
        assert!(!is_remote_dir_dest("other", &entries));
    }
}
