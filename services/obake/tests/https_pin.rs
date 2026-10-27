// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! HTTPS pin: missing fingerprint fails closed; a mismatch never serves HTTP.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::Body;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use obake::{Client, ClientError, Endpoint};
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::ServerConfig;
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

fn fingerprint(der: &[u8]) -> String {
    let digest = Sha256::digest(der);
    let mut hex = String::from("sha256:");
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

async fn serve_pinned() -> (std::net::SocketAddr, String, Arc<AtomicUsize>) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let key = KeyPair::generate().unwrap();
    let params = CertificateParams::new(vec!["localhost".into()]).unwrap();
    let cert = params.self_signed(&key).unwrap();
    let der = cert.der().to_vec();
    let fp = fingerprint(&der);
    let certs = vec![CertificateDer::from(der)];
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
    let server = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(server));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = hits.clone();
    tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                break;
            };
            let acceptor = acceptor.clone();
            let hits = hits2.clone();
            tokio::spawn(async move {
                let Ok(tls) = acceptor.accept(tcp).await else {
                    return;
                };
                let io = TokioIo::new(tls);
                let svc = service_fn(move |_req| {
                    hits.fetch_add(1, Ordering::SeqCst);
                    async move {
                        Ok::<_, std::convert::Infallible>(
                            hyper::Response::new(Body::from(r#"{"data":{"rigs":[]}}"#)),
                        )
                    }
                });
                let _ = http1::Builder::new().serve_connection(io, svc).await;
            });
        }
    });
    (addr, fp, hits)
}

#[tokio::test]
async fn missing_fingerprint_does_not_query() {
    let client = Client::new(Endpoint::Http {
        url: "https://studio.local:9477".into(),
        token: Some("tok".into()),
        fingerprint: None,
    });
    let err = client.rigs().await.unwrap_err();
    assert!(
        matches!(err, ClientError::Message(ref m) if m.contains("fingerprint")),
        "{err}"
    );
}

#[tokio::test]
async fn pin_match_serves_and_mismatch_does_not() {
    let (addr, fp, hits) = serve_pinned().await;
    let url = format!("https://127.0.0.1:{}/", addr.port());
    let bad = Client::new(Endpoint::Http {
        url: url.clone(),
        token: None,
        fingerprint: Some(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        ),
    });
    assert!(bad.rigs().await.is_err());
    assert_eq!(hits.load(Ordering::SeqCst), 0);

    let good = Client::new(Endpoint::Http {
        url,
        token: None,
        fingerprint: Some(fp),
    });
    let data = good.rigs().await.unwrap();
    assert!(data.get("rigs").is_some(), "{data}");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}
