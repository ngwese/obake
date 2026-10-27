// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! obake client against a live obake-session TLS listener.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use obake::auth;
use obake::{save_config, ClientConfig, HostConfig};
use obake::{Client, Endpoint, Resolved};
use obake_session::launch::FakeLauncher;
use obake_session::links::FakeGraph;
use obake_session::paths::Paths;
use obake_session::service::Service;
use tokio::net::TcpListener;

#[tokio::test]
async fn credential_approve_pin_and_rotate() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();
    let service = Service::new(
        paths,
        Arc::new(FakeLauncher::default()),
        Arc::new(FakeGraph::default()),
    )
    .unwrap();
    let sock = service.paths().api_socket();
    let svc_u = service.clone();
    let sock_path = sock.clone();
    tokio::spawn(async move {
        obake_session::graphql::serve_unix(svc_u, &sock_path)
            .await
            .ok();
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let svc_t = service.clone();
    tokio::spawn(async move {
        obake_session::graphql::serve_tls_on(svc_t, listener)
            .await
            .ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    let unix = Client::new(Endpoint::Unix {
        path: sock.display().to_string(),
    });
    let raw = unix.get_raw("/api/v0/credential").await.unwrap();
    let cred: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(cred["token"], service.token());
    assert_eq!(cred["fingerprint"], service.fingerprint());
    assert_eq!(cred["port"], addr.port());

    let started = service.start_login().unwrap();
    let fp = service.fingerprint();
    let bearer = service.token();
    let resolved = Resolved {
        endpoint: format!("https://127.0.0.1:{}", addr.port()),
        token: Some(bearer.clone()),
        fingerprint: Some(fp.clone()),
        host_name: Some("127.0.0.1".into()),
    };
    auth::approve(&resolved, &started.user_code).await.unwrap();
    assert!(service.poll_login(&started.poll_secret).unwrap());

    let client = Client::from_resolved(&resolved);
    client.rigs().await.unwrap();

    let cfg_path = dir.path().join("config.yaml");
    let mut cfg = ClientConfig::default();
    cfg.default = Some("127.0.0.1".into());
    cfg.hosts.insert(
        "127.0.0.1".into(),
        HostConfig {
            endpoint: resolved.endpoint.clone(),
            token: bearer.clone(),
            fingerprint: Some(fp.clone()),
        },
    );
    save_config(&cfg_path, &cfg).unwrap();
    auth::rotate(&cfg_path, &resolved).await.unwrap();
    let loaded = obake::load_config(&cfg_path).unwrap();
    let stored = &loaded.hosts["127.0.0.1"];
    assert_ne!(stored.token, bearer);
    assert_eq!(stored.fingerprint.as_deref(), Some(fp.as_str()));
    assert_eq!(stored.token, service.token());
    let mode = std::fs::metadata(&cfg_path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);

    let stale = Client::new(Endpoint::Http {
        url: resolved.endpoint.clone(),
        token: Some(bearer),
        fingerprint: Some(fp),
    });
    assert!(stale.rigs().await.is_err());
    client.rigs().await.unwrap_err();
    let fresh = Client::new(Endpoint::Http {
        url: resolved.endpoint,
        token: Some(service.token()),
        fingerprint: Some(service.fingerprint()),
    });
    fresh.rigs().await.unwrap();
}
