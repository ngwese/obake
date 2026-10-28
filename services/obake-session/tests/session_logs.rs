// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Active session log HTTP streaming.

use axum::body::Body;
use http_body_util::BodyExt;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use obake_session::launch::{singularity_name, FakeLauncher};
use obake_session::links::FakeGraph;
use obake_session::logs::MapLogPaths;
use obake_session::paths::Paths;
use obake_session::service::Service;
use obake_session::store::SessionMode;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tokio::net::TcpListener;

fn setup_with_logs() -> (tempfile::TempDir, Arc<Service>, String) {
    let dir = tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();
    fs::write(
        paths.rig_file("tone"),
        r#"
instances:
  tone:
    shape: chuck
    command: ["tone.ck"]
    roles: { out: [left, right], in: [] }
  pad:
    shape: chuck
    command: ["pad.ck"]
    roles: { out: [pad], in: [] }
links: []
"#,
    )
    .unwrap();
    fs::create_dir_all(&paths.default_image_dir).unwrap();
    fs::write(paths.default_image_dir.join("chuck.sif"), "fake").unwrap();

    let map = Arc::new(MapLogPaths::new());
    let service = Service::new_with_logs(
        paths,
        Arc::new(FakeLauncher::new()),
        Arc::new(FakeGraph::new()),
        map.clone(),
        Some(map),
    )
    .unwrap();
    let session = service
        .start_session("tone", SessionMode::Fresh, None)
        .unwrap();
    let sid = session.record.id.clone();

    let log_dir = dir.path().join("logs");
    fs::create_dir_all(&log_dir).unwrap();
    for name in ["tone", "pad"] {
        let sin = singularity_name(&sid, name);
        let out = log_dir.join(format!("{sin}.out"));
        let err = log_dir.join(format!("{sin}.err"));
        fs::write(&out, "").unwrap();
        fs::write(&err, format!("{name}-line\n")).unwrap();
        service
            .register_log_files_for_test(&sin, out, err)
            .unwrap();
    }

    (dir, service, sid)
}

async fn http_get(addr: std::net::SocketAddr, path_and_query: &str) -> (u16, String) {
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = hyper::Request::builder()
        .method("GET")
        .uri(format!("http://localhost{path_and_query}"))
        .body(Body::empty())
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    let status = res.status().as_u16();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&body).into_owned())
}

#[test]
fn rejects_no_session_and_unknown_instance() {
    let dir = tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();
    let map = Arc::new(MapLogPaths::new());
    let service = Service::new_with_logs(
        paths,
        Arc::new(FakeLauncher::new()),
        Arc::new(FakeGraph::new()),
        map.clone(),
        Some(map),
    )
    .unwrap();
    let err = service.session_log_targets(&[]).unwrap_err().to_string();
    assert!(err.contains("no active session"));

    let (_dir, service, _sid) = setup_with_logs();
    let err = service
        .session_log_targets(&["missing".into()])
        .unwrap_err()
        .to_string();
    assert!(err.contains("unknown instance"));
}

#[tokio::test]
async fn http_dump_multiplexes_and_filters() {
    let (_dir, service, _sid) = setup_with_logs();
    let app = obake_session::graphql::router_for_tests(service, false);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (status, all) = http_get(addr, "/api/v0/sessions/current/logs").await;
    assert_eq!(status, 200);
    assert!(all.contains("[tone] tone-line\n"), "{all}");
    assert!(all.contains("[pad] pad-line\n"), "{all}");

    let (status, filtered) =
        http_get(addr, "/api/v0/sessions/current/logs?instance=tone").await;
    assert_eq!(status, 200);
    assert!(filtered.contains("[tone] tone-line\n"));
    assert!(!filtered.contains("[pad]"));

    let (status, _) = http_get(addr, "/api/v0/sessions/current/logs?instance=nope").await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn http_follow_sees_append_and_ends_on_stop() {
    let (_dir, service, sid) = setup_with_logs();
    let targets = service.session_log_targets(&["tone".into()]).unwrap();
    let err_path = targets[0].files.err.clone();

    let app = obake_session::graphql::router_for_tests(service.clone(), false);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = hyper::Request::builder()
        .method("GET")
        .uri("http://localhost/api/v0/sessions/current/logs?follow=1&instance=tone")
        .body(Body::empty())
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    assert_eq!(res.status(), 200);
    let mut body = res.into_body();

    let service2 = service.clone();
    let err_path2 = err_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let mut f = fs::OpenOptions::new()
            .append(true)
            .open(&err_path2)
            .unwrap();
        f.write_all(b"after\n").unwrap();
        tokio::time::sleep(Duration::from_millis(600)).await;
        let _ = service2.stop_session();
        let _ = sid; // keep sid in scope for clarity
    });

    let mut collected = Vec::new();
    loop {
        match tokio::time::timeout(Duration::from_secs(4), body.frame()).await {
            Ok(Some(Ok(frame))) => {
                if let Ok(data) = frame.into_data() {
                    collected.extend_from_slice(&data);
                }
            }
            Ok(Some(Err(_))) | Ok(None) | Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&collected);
    assert!(text.contains("[tone] tone-line\n"), "{text}");
    assert!(text.contains("[tone] after\n"), "{text}");
}
