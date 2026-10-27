// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Browser cookie, device-code approval, and bearer rotation over TLS.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use http_body_util::BodyExt;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use obake_session::auth::browser_cookie;
use obake_session::launch::FakeLauncher;
use obake_session::links::FakeGraph;
use obake_session::paths::Paths;
use obake_session::service::Service;
use tokio::net::{TcpListener, UnixStream};
use tokio_rustls::TlsConnector;

#[path = "common/pin.rs"]
mod pin;

fn setup() -> (tempfile::TempDir, Arc<Service>) {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();
    std::fs::write(paths.host_yaml(), "listen: \"127.0.0.1:9\"\n").unwrap();
    let service = Service::new(
        paths,
        Arc::new(FakeLauncher::default()),
        Arc::new(FakeGraph::default()),
    )
    .unwrap();
    (dir, service)
}

async fn serve(service: Arc<Service>) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        obake_session::graphql::serve_tls_on(service, listener)
            .await
            .ok();
    });
    tokio::time::sleep(Duration::from_millis(80)).await;
    addr
}

#[derive(Debug)]
struct Reply {
    status: u16,
    body: String,
    set_cookie: Option<String>,
}

async fn tls_exchange(
    addr: std::net::SocketAddr,
    fingerprint: &str,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    cookie: Option<&str>,
    body: Option<&str>,
) -> Reply {
    let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
    let connector = TlsConnector::from(Arc::new(pin::client_config(fingerprint)));
    let tls = connector
        .connect(pin::server_name(), tcp)
        .await
        .unwrap();
    let io = TokioIo::new(tls);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let mut builder = hyper::Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "localhost");
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    if let Some(t) = bearer {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    if let Some(c) = cookie {
        builder = builder.header("cookie", format!("obake_session={c}"));
    }
    let req = builder
        .body(Body::from(body.unwrap_or("").to_string()))
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    let status = res.status().as_u16();
    let set_cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        body: String::from_utf8_lossy(&bytes).into_owned(),
        set_cookie,
    }
}

async fn unix_graphql(service: &Service, query: &str) -> String {
    let sock = service.paths().api_socket();
    let stream = UnixStream::connect(&sock).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = hyper::Request::builder()
        .method("POST")
        .uri("http://localhost/api/v0/")
        .header("content-type", "application/json")
        .body(Body::from(query.to_string()))
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[tokio::test]
async fn cookie_authorizes_and_cannot_rotate() {
    let (_dir, service) = setup();
    let addr = serve(service.clone()).await;
    let fp = service.fingerprint();
    let bearer = service.token();
    let query = r#"{"query":"{ rigs { name } }"}"#;

    let denied = tls_exchange(addr, &fp, "POST", "https://localhost/api/v0/", None, None, Some(query)).await;
    assert_eq!(denied.status, 401);

    let cookie = browser_cookie(&bearer);
    let ok = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/",
        None,
        Some(&cookie),
        Some(query),
    )
    .await;
    assert_eq!(ok.status, 200, "{}", ok.body);
    assert!(ok.body.contains("rigs"), "{}", ok.body);

    let rotate = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/",
        None,
        Some(&cookie),
        Some(r#"{"query":"mutation { rotateAuth }"}"#),
    )
    .await;
    assert!(rotate.body.contains("errors"), "{}", rotate.body);
    assert_eq!(service.token(), bearer);

    let cred = tls_exchange(
        addr,
        &fp,
        "GET",
        "https://localhost/api/v0/credential",
        Some(&bearer),
        None,
        None,
    )
    .await;
    assert_ne!(cred.status, 200);
    assert!(!cred.body.contains(&bearer));
}

#[tokio::test]
async fn device_code_approve_from_socket_and_bearer() {
    let (_dir, service) = setup();
    let sock = service.paths().api_socket();
    let svc_u = service.clone();
    let sock_path = sock.clone();
    tokio::spawn(async move {
        obake_session::graphql::serve_unix(svc_u, &sock_path)
            .await
            .ok();
    });
    let addr = serve(service.clone()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let fp = service.fingerprint();

    let started = tls_exchange(addr, &fp, "POST", "https://localhost/api/v0/login/start", None, None, Some(""))
        .await;
    let start_json: serde_json::Value = serde_json::from_str(&started.body).unwrap();
    let code = start_json["userCode"].as_str().unwrap();
    let secret = start_json["pollSecret"].as_str().unwrap();

    let stray = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/login/poll",
        None,
        None,
        Some(&format!(r#"{{"pollSecret":"{code}"}}"#)),
    )
    .await;
    assert!(stray.set_cookie.is_none(), "{stray:?}");

    let approved = unix_graphql(
        &service,
        &format!(r#"{{"query":"mutation($c: String!) {{ approveLogin(code: $c) }}","variables":{{"c":"{code}"}}}}"#),
    )
    .await;
    assert!(approved.contains("true"), "{approved}");

    let polled = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/login/poll",
        None,
        None,
        Some(&format!(r#"{{"pollSecret":"{secret}"}}"#)),
    )
    .await;
    assert_eq!(polled.status, 200, "{}", polled.body);
    let cookie_hdr = polled.set_cookie.expect("set-cookie");
    assert!(cookie_hdr.contains("obake_session="));

    let started = tls_exchange(addr, &fp, "POST", "https://localhost/api/v0/login/start", None, None, Some(""))
        .await;
    let start_json: serde_json::Value = serde_json::from_str(&started.body).unwrap();
    let code = start_json["userCode"].as_str().unwrap();
    let secret = start_json["pollSecret"].as_str().unwrap();
    let bearer = service.token();
    let approved = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/",
        Some(&bearer),
        None,
        Some(&format!(
            r#"{{"query":"mutation($c: String!) {{ approveLogin(code: $c) }}","variables":{{"c":"{code}"}}}}"#
        )),
    )
    .await;
    assert!(approved.body.contains("true"), "{}", approved.body);
    let polled = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/login/poll",
        None,
        None,
        Some(&format!(r#"{{"pollSecret":"{secret}"}}"#)),
    )
    .await;
    assert!(polled.set_cookie.is_some(), "{}", polled.body);
}

#[tokio::test]
async fn expired_and_restarted_codes_are_rejected() {
    let (dir, service) = setup();
    let started = service.start_login().unwrap();
    service.backdate_pending_logins(Duration::from_secs(16 * 60));
    let err = service.approve_login(&started.user_code).unwrap_err();
    assert!(err.to_string().contains("expired") || err.to_string().contains("unknown"));

    let started = service.start_login().unwrap();
    drop(service);
    let paths = Paths::under(dir.path());
    let restarted = Service::new(
        paths,
        Arc::new(FakeLauncher::default()),
        Arc::new(FakeGraph::default()),
    )
    .unwrap();
    assert!(restarted.approve_login(&started.user_code).is_err());
}

#[tokio::test]
async fn rotate_invalidates_bearer_and_cookie_keeps_certificate() {
    let (_dir, service) = setup();
    let sock = service.paths().api_socket();
    let svc_u = service.clone();
    let sock_path = sock.clone();
    tokio::spawn(async move {
        obake_session::graphql::serve_unix(svc_u, &sock_path)
            .await
            .ok();
    });
    let addr = serve(service.clone()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let fp = service.fingerprint();
    let old = service.token();
    let cookie = browser_cookie(&old);

    let stream = UnixStream::connect(&sock).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = hyper::Request::builder()
        .method("GET")
        .uri("http://localhost/api/v0/credential")
        .body(Body::empty())
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(doc["token"], old);
    assert_eq!(doc["fingerprint"], fp);

    let rotated = unix_graphql(
        &service,
        r#"{"query":"mutation { rotateAuth }"}"#,
    )
    .await;
    assert!(rotated.contains("rotateAuth"), "{rotated}");
    let new_token = service.token();
    assert_ne!(new_token, old);
    assert_eq!(service.fingerprint(), fp);

    let old_bearer = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/",
        Some(&old),
        None,
        Some(r#"{"query":"{ rigs { name } }"}"#),
    )
    .await;
    assert_eq!(old_bearer.status, 401);
    let old_cookie = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/",
        None,
        Some(&cookie),
        Some(r#"{"query":"{ rigs { name } }"}"#),
    )
    .await;
    assert_eq!(old_cookie.status, 401);
    let fresh = tls_exchange(
        addr,
        &fp,
        "POST",
        "https://localhost/api/v0/",
        Some(&new_token),
        None,
        Some(r#"{"query":"{ rigs { name } }"}"#),
    )
    .await;
    assert_eq!(fresh.status, 200, "{}", fresh.body);
}
