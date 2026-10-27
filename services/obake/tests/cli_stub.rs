// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Headless CLI tests against a stub GraphQL / HTTP endpoint (no obake-session).

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::routing::{get, post};
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
    files: Arc<Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

async fn graphql_handler(
    State(stub): State<Stub>,
    req: axum::http::Request<Body>,
) -> axum::Json<Value> {
    let bytes = req.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let query = body["query"].as_str().unwrap_or("").to_string();
    let op = if query.contains("sessionFiles") {
        "files"
    } else if query.contains("rigs") && !query.contains("installRig") {
        "rigs"
    } else if query.contains("installRig") {
        "install-rig"
    } else if query.contains("removeRig") {
        "remove-rig"
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
    } else if query.contains("graph") {
        "graph"
    } else if query.contains("session") {
        "session"
    } else {
        "unknown"
    };
    stub.ops.lock().unwrap().push(op.to_string());
    let data = if op == "session" {
        json!({
            "session": {
                "id": "sess-active",
                "rig": "tone",
                "mode": "FRESH",
                "phase": "RUNNING"
            }
        })
    } else if op == "sessions" {
        json!({
            "sessions": [{
                "id": "sess-active",
                "rig": "tone",
                "mode": "FRESH",
                "forkedFrom": null
            }, {
                "id": "sess-old",
                "rig": "quartet",
                "mode": "FRESH",
                "forkedFrom": null
            }]
        })
    } else {
        json!({ "ok": true, "rigs": [], "session": null, "sessionFiles": [] })
    };
    axum::Json(json!({ "data": data }))
}

async fn get_file(
    State(stub): State<Stub>,
    Path((session_id, path)): Path<(String, String)>,
) -> impl axum::response::IntoResponse {
    stub.ops
        .lock()
        .unwrap()
        .push(format!("get-file:{session_id}:{path}"));
    let key = format!("{session_id}:{path}");
    let files = stub.files.lock().unwrap();
    match files.get(&key) {
        Some(bytes) => (
            axum::http::StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
            bytes.clone(),
        )
            .into_response(),
        None => (
            axum::http::StatusCode::NOT_FOUND,
            "missing".to_string(),
        )
            .into_response(),
    }
}

async fn put_file(
    State(stub): State<Stub>,
    Path((session_id, path)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> impl axum::response::IntoResponse {
    stub.ops
        .lock()
        .unwrap()
        .push(format!("put-file:{session_id}:{path}"));
    let key = format!("{session_id}:{path}");
    stub.files.lock().unwrap().insert(key, body.to_vec());
    axum::http::StatusCode::NO_CONTENT
}

use axum::response::IntoResponse;

async fn spawn_stub() -> (SocketAddr, Arc<Mutex<Vec<String>>>, Stub) {
    let stub = Stub::default();
    stub.files.lock().unwrap().insert(
        "sess-active:take.wav".into(),
        b"WAVDATA".to_vec(),
    );
    let ops = stub.ops.clone();
    let files = Router::new()
        .route(
            "/api/v0/sessions/{session_id}/files/{*path}",
            get(get_file).put(put_file),
        )
        .layer(DefaultBodyLimit::disable());
    let app = Router::new()
        .route("/api/v0/", post(graphql_handler))
        .merge(files)
        .with_state(stub.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    (addr, ops, stub)
}

#[tokio::test]
async fn each_subcommand_records_one_operation() {
    let (addr, ops, _stub) = spawn_stub().await;
    let client = Client::new(Endpoint::Http {
        url: format!("http://{addr}/"),
        token: None,
        fingerprint: None,
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
    let _ = client.delete_session("id1").await;
    let _ = client.graph().await;

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
            "delete-session",
            "graph",
        ]
    );
}

#[tokio::test]
async fn file_get_put_http_and_session_flag() {
    use tempfile::tempdir;
    use tokio::io::AsyncWriteExt;

    let (addr, ops, _stub) = spawn_stub().await;
    let client = Client::new(Endpoint::Http {
        url: format!("http://{addr}/"),
        token: None,
        fingerprint: None,
    });

    let dir = tempdir().unwrap();

    // get to file
    let dest = dir.path().join("out.wav");
    let mut file = tokio::fs::File::create(&dest).await.unwrap();
    let n = client
        .get_file_to("sess-active", "take.wav", &mut file)
        .await
        .unwrap();
    file.flush().await.unwrap();
    drop(file);
    assert_eq!(n, 7);
    assert_eq!(tokio::fs::read(&dest).await.unwrap(), b"WAVDATA");

    // get to stdout-like sink
    let mut sink: Vec<u8> = Vec::new();
    let n = client
        .get_file_to("sess-active", "take.wav", &mut sink)
        .await
        .unwrap();
    assert_eq!(n, 7);
    assert_eq!(sink, b"WAVDATA");

    // put from path
    let local = dir.path().join("patch.ck");
    tokio::fs::write(&local, b"SinOsc s;").await.unwrap();
    let n = client
        .put_file_from_path("sess-old", "patch.ck", &local)
        .await
        .unwrap();
    assert_eq!(n, 9);

    let mut got: Vec<u8> = Vec::new();
    client
        .get_file_to("sess-old", "patch.ck", &mut got)
        .await
        .unwrap();
    assert_eq!(got, b"SinOsc s;");

    // multi-MiB round-trip through streaming API
    let big_local = dir.path().join("big.bin");
    let size = 9 * 1024 * 1024;
    {
        let mut f = tokio::fs::File::create(&big_local).await.unwrap();
        let chunk = vec![0x5Au8; 1024 * 1024];
        let mut left = size;
        while left > 0 {
            let n = left.min(chunk.len());
            f.write_all(&chunk[..n]).await.unwrap();
            left -= n;
        }
        f.flush().await.unwrap();
    }
    let n = client
        .put_file_from_path("sess-old", "big.bin", &big_local)
        .await
        .unwrap();
    assert_eq!(n, size as u64);
    let big_out = dir.path().join("big.out");
    let mut out = tokio::fs::File::create(&big_out).await.unwrap();
    let n = client
        .get_file_to("sess-old", "big.bin", &mut out)
        .await
        .unwrap();
    out.flush().await.unwrap();
    assert_eq!(n, size as u64);
    assert_eq!(
        tokio::fs::metadata(&big_out).await.unwrap().len(),
        size as u64
    );

    let recorded = ops.lock().unwrap().clone();
    assert!(recorded.iter().any(|o| o == "get-file:sess-active:take.wav"));
    assert!(recorded.iter().any(|o| o == "put-file:sess-old:patch.ck"));
    assert!(recorded.iter().any(|o| o == "put-file:sess-old:big.bin"));
    assert!(recorded.iter().any(|o| o == "get-file:sess-old:big.bin"));
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
