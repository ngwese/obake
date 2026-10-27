// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Self-signed host certificate for the session TLS listener.

use crate::paths::Paths;
use rcgen::{CertificateParams, DnType, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::Arc;
use thiserror::Error;

/// `sha256:` plus the hex SHA-256 of the certificate DER.
pub fn fingerprint_der(der: &[u8]) -> String {
    let digest = Sha256::digest(der);
    let mut hex = String::with_capacity(7 + digest.len() * 2);
    hex.push_str("sha256:");
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Load or create the host key and certificate. Files are mode 0600.
pub fn ensure_host_cert(paths: &Paths) -> Result<(Arc<ServerConfig>, String), TlsError> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    fs::create_dir_all(&paths.config_dir)?;
    let cert_path = paths.tls_cert_file();
    let key_path = paths.tls_key_file();
    if !cert_path.is_file() || !key_path.is_file() {
        write_new_cert(&cert_path, &key_path)?;
    }
    load_cert(&cert_path, &key_path)
}

fn system_hostname() -> String {
    let mut buf = [0u8; 256];
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if rc != 0 {
        return "obake".into();
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let name = String::from_utf8_lossy(&buf[..end]).trim().to_string();
    if name.is_empty() {
        "obake".into()
    } else {
        name
    }
}

fn write_new_cert(cert_path: &std::path::Path, key_path: &std::path::Path) -> Result<(), TlsError> {
    let hostname = system_hostname();
    let local = format!("{hostname}.local");
    let key_pair = KeyPair::generate()?;
    let mut params = CertificateParams::new(vec![hostname.clone(), local])?;
    params
        .distinguished_name
        .push(DnType::CommonName, hostname);
    let cert = params.self_signed(&key_pair)?;
    write_secret(cert_path, cert.pem().as_bytes())?;
    write_secret(key_path, key_pair.serialize_pem().as_bytes())?;
    Ok(())
}

fn write_secret(path: &std::path::Path, bytes: &[u8]) -> Result<(), TlsError> {
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}

fn load_cert(
    cert_path: &std::path::Path,
    key_path: &std::path::Path,
) -> Result<(Arc<ServerConfig>, String), TlsError> {
    let mut cert_reader = BufReader::new(File::open(cert_path)?);
    let cert_ders: Vec<CertificateDer<'static>> = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(CertificateDer::from)
        .collect();
    let leaf = cert_ders
        .first()
        .cloned()
        .ok_or_else(|| TlsError::Message("certificate file is empty".into()))?;
    let fingerprint = fingerprint_der(leaf.as_ref());
    let mut key_reader = BufReader::new(File::open(key_path)?);
    let key = private_key(&mut key_reader)?.ok_or_else(|| {
        TlsError::Message("private key file is empty".into())
    })?;
    let key = match key {
        PrivateKeyDer::Pkcs8(k) => PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(k.secret_pkcs8_der().to_vec())),
        other => other,
    };
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_ders, key)
        .map_err(|e| TlsError::Message(e.to_string()))?;
    Ok((Arc::new(config), fingerprint))
}

/// Certificate errors.
#[derive(Debug, Error)]
pub enum TlsError {
    /// I/O.
    #[error("tls i/o: {0}")]
    Io(#[from] std::io::Error),
    /// Generation or parse failure.
    #[error("{0}")]
    Message(String),
    /// rcgen.
    #[error("tls generate: {0}")]
    Rcgen(#[from] rcgen::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Paths;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn creates_once_and_reuses_fingerprint() {
        let dir = tempdir().unwrap();
        let paths = Paths::under(dir.path());
        paths.ensure().unwrap();
        let (_cfg, first) = ensure_host_cert(&paths).unwrap();
        let (_cfg2, second) = ensure_host_cert(&paths).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("sha256:"));
        let mode = fs::metadata(paths.tls_key_file()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);

        let mut reader = BufReader::new(File::open(paths.tls_cert_file()).unwrap());
        let der = certs(&mut reader).next().unwrap().unwrap();
        let host = system_hostname();
        let local = format!("{host}.local");
        assert!(
            der.windows(host.len()).any(|w| w == host.as_bytes()),
            "certificate DER missing {host}"
        );
        assert!(
            der.windows(local.len()).any(|w| w == local.as_bytes()),
            "certificate DER missing {local}"
        );
    }
}
