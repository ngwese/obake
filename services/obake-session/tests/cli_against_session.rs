// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Integration: GraphQL client (as used by `obake`) against a live session
//! socket. Ignored by default; not part of `cargo test -p obake`.

use axum::body::Body;
use http_body_util::BodyExt;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use obake_session::launch::FakeLauncher;
use obake_session::links::FakeGraph;
use obake_session::paths::Paths;
use obake_session::service::Service;
use serde_json::json;
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::UnixStream;

#[tokio::test]
#[ignore = "run with --ignored: obake-style client against session socket"]
async fn start_via_unix_socket_activates_session() {
    let dir = tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();
    fs::write(
        paths.rig_file("quartet"),
        r#"
instances:
  lead:
    shape: chuck
    command: ["lead.ck"]
    roles:
      out: [melody]
      in: []
links: []
"#,
    )
    .unwrap();
    fs::create_dir_all(&paths.default_image_dir).unwrap();
    fs::write(paths.default_image_dir.join("chuck.sif"), "x").unwrap();

    let launcher = Arc::new(FakeLauncher::new());
    let graph = Arc::new(FakeGraph::new());
    let service = Service::new(paths.clone(), launcher, graph).unwrap();
    let sock = paths.api_socket();
    let svc = service.clone();
    let sock2 = sock.clone();
    tokio::spawn(async move {
        let _ = obake_session::graphql::serve_unix(svc, &sock2).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    let stream = UnixStream::connect(&sock).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let body = json!({
        "query": "mutation($i: StartSessionInput!) { startSession(input: $i) { id rig } }",
        "variables": { "i": { "rig": "quartet", "mode": "FRESH" } }
    });
    let req = hyper::Request::builder()
        .method("POST")
        .uri("http://localhost/api/v0/")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    assert_eq!(res.status(), 200);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value["errors"].is_null() || value["errors"].as_array().unwrap().is_empty());
    assert!(service.session().is_some());
    assert_eq!(
        service.session().unwrap().record.rig,
        "quartet"
    );
}
