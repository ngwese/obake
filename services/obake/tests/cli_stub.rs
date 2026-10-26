// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Headless CLI tests against a stub GraphQL endpoint (no obake-session).

use axum::body::Body;
use axum::extract::State;
use axum::routing::post;
use axum::Router;
use http_body_util::BodyExt;
use obake::{Client, Endpoint};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[derive(Clone, Default)]
struct Stub {
    ops: Arc<Mutex<Vec<String>>>,
}

async fn handler(State(stub): State<Stub>, req: axum::http::Request<Body>) -> axum::Json<Value> {
    let bytes = req.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let query = body["query"].as_str().unwrap_or("").to_string();
    let op = if query.contains("sessionFiles") {
        "files"
    } else if query.contains("sessionFile") {
        "file"
    } else if query.contains("rigs") && !query.contains("installRig") {
        "rigs"
    } else if query.contains("rig(") {
        "rig"
    } else if query.contains("shapes") {
        "shapes"
    } else if query.contains("sessions(") {
        "sessions"
    } else if query.contains("startSession") {
        "start"
    } else if query.contains("stopSession") {
        "stop"
    } else if query.contains("setSystemLinks") {
        "links"
    } else if query.contains("deleteSession") {
        "delete-session"
    } else if query.contains("session") {
        "session"
    } else {
        "unknown"
    };
    stub.ops.lock().unwrap().push(op.to_string());
    axum::Json(json!({ "data": { "ok": true, "rigs": [], "session": null } }))
}

async fn spawn_stub() -> (SocketAddr, Arc<Mutex<Vec<String>>>) {
    let stub = Stub::default();
    let ops = stub.ops.clone();
    let app = Router::new().route("/", post(handler)).with_state(stub);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    (addr, ops)
}

#[tokio::test]
async fn each_subcommand_records_one_operation() {
    let (addr, ops) = spawn_stub().await;
    let client = Client::new(Endpoint::Http {
        url: format!("http://{addr}/"),
        token: None,
    });

    let _ = client.rigs().await;
    let _ = client.rig("quartet").await;
    let _ = client.shapes().await;
    let _ = client.sessions(Some("quartet")).await;
    let _ = client.session().await;
    let _ = client.start("quartet", "FRESH", None).await;
    let _ = client.stop().await;
    let _ = client.set_links(json!([])).await;
    let _ = client.files("id1").await;
    let _ = client.file("id1", "take.wav").await;
    let _ = client.delete_session("id1").await;

    let recorded = ops.lock().unwrap().clone();
    assert_eq!(
        recorded,
        vec![
            "rigs",
            "rig",
            "shapes",
            "sessions",
            "session",
            "start",
            "stop",
            "links",
            "files",
            "file",
            "delete-session",
        ]
    );
}

#[test]
fn config_resolution_studio_spare() {
    use obake::{load_config, resolve_endpoint};
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    fs::write(
        &path,
        r#"
default: studio
hosts:
  studio:
    endpoint: http://192.168.60.10:9477
    token: tok-studio
  spare:
    endpoint: http://192.168.60.11:9477
    token: tok-spare
"#,
    )
    .unwrap();
    let cfg = load_config(&path).unwrap();
    let r = resolve_endpoint(&cfg, None, None, None, None).unwrap();
    assert_eq!(r.endpoint, "http://192.168.60.10:9477");
    assert_eq!(r.token.as_deref(), Some("tok-studio"));
    let r = resolve_endpoint(&cfg, Some("spare"), None, None, None).unwrap();
    assert_eq!(r.endpoint, "http://192.168.60.11:9477");
    let r = resolve_endpoint(&cfg, None, Some("http://x:9"), Some("t"), None).unwrap();
    assert_eq!(r.endpoint, "http://x:9");
    assert_eq!(r.token.as_deref(), Some("t"));
    assert!(resolve_endpoint(&cfg, Some("nope"), None, None, None).is_err());
    let empty = obake::ClientConfig::default();
    let r = resolve_endpoint(&empty, None, None, None, Some(dir.path())).unwrap();
    assert!(r.endpoint.contains("unix://"));
}
