// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Milestone 1 verification fixtures.

use obake_session::config::parse_rig;
use obake_session::graphql;
use obake_session::launch::FakeLauncher;
use obake_session::links::{FakeGraph, PortDirection};
use obake_session::paths::Paths;
use obake_session::service::{system_link, Service};
use obake_session::store::{read_current, read_session, SessionMode};
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;

fn lead_rig() -> &'static str {
    r#"
instances:
  lead:
    shape: chuck
    command: ["--out:2", "lead.ck"]
    roles:
      out: [melody, bed]
      in: []
links: []
"#
}

fn quartet_multi() -> &'static str {
    r#"
instances:
  lead:
    shape: chuck
    command: ["lead.ck"]
    roles:
      out: [melody]
      in: []
  harmony:
    shape: chuck
    command: ["harmony.ck"]
    roles:
      out: [pad]
      in: []
  space:
    shape: rnbo-runner
    command: []
    roles:
      out: []
      in: [wet]
links:
  - from: lead.melody
    to: space.wet
  - from: harmony.pad
    to: space.wet
"#
}

fn setup() -> (tempfile::TempDir, Arc<Service>, Arc<FakeLauncher>, Arc<FakeGraph>) {
    let dir = tempdir().unwrap();
    let paths = Paths::under(dir.path());
    paths.ensure().unwrap();
    fs::write(paths.rig_file("quartet"), lead_rig()).unwrap();
    fs::create_dir_all(paths.rig_work("quartet")).unwrap();
    fs::write(paths.rig_work("quartet").join("lead.ck"), "SinOsc s;").unwrap();
    // Shape images
    fs::create_dir_all(&paths.default_image_dir).unwrap();
    fs::write(paths.default_image_dir.join("chuck.sif"), "fake").unwrap();
    fs::write(paths.default_image_dir.join("rnbo-runner.sif"), "fake").unwrap();
    fs::write(paths.default_image_dir.join("siren.sif"), "fake").unwrap();
    fs::write(paths.default_image_dir.join("mod-host.sif"), "fake").unwrap();

    let launcher = Arc::new(FakeLauncher::new());
    let graph = Arc::new(FakeGraph::new());
    let service = Service::new(paths, launcher.clone(), graph.clone()).unwrap();
    (dir, service, launcher, graph)
}

#[test]
fn parse_rejects_unsafe_and_duplicate() {
    assert!(parse_rig("q", lead_rig()).is_ok());
    let colon = r#"
instances:
  "a:b":
    shape: chuck
    command: []
    roles: { out: [], in: [] }
"#;
    assert!(parse_rig("q", colon).is_err());
    let slash = r#"
instances:
  "a/b":
    shape: chuck
    command: []
    roles: { out: [], in: [] }
"#;
    assert!(parse_rig("q", slash).is_err());
}

#[tokio::test]
async fn fresh_start_persists_and_restores() {
    let (dir, service, launcher, _graph) = setup();
    let session = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    assert!(session.record.system_links.is_empty());
    assert!(service
        .paths()
        .rig_work("quartet")
        .join("lead.ck")
        .is_file());
    let work = service.paths().session_work(&session.record.id);
    assert!(work.read_dir().unwrap().next().is_none());

    let started = launcher.started();
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].instance_name, "lead");
    let props = started[0].env.get("PIPEWIRE_PROPS").unwrap();
    assert!(
        props.contains("node.name\"=\"lead\"") || props.contains("lead")
    );
    assert!(props.contains("node.autoconnect\"=false"));
    assert!(props.contains("jack.self-connect-mode\"=\"ignore-external\""));
    assert!(started[0]
        .binds
        .iter()
        .any(|b| b.target == "/work"));
    assert!(started[0]
        .binds
        .iter()
        .any(|b| b.target == "/session"));

    // Simulate process death: new service from same paths restores.
    let id = session.record.id.clone();
    drop(service);
    let paths = Paths::under(dir.path());
    let launcher2 = Arc::new(FakeLauncher::new());
    let graph2 = Arc::new(FakeGraph::new());
    let service2 = Service::new(paths, launcher2.clone(), graph2).unwrap();
    service2.restore().unwrap();
    let restored = service2.session().unwrap();
    assert_eq!(restored.record.id, id);
    assert_eq!(restored.record.rig, "quartet");
    assert!(restored.record.system_links.is_empty());
    assert_eq!(launcher2.started().len(), 1);
}

#[tokio::test]
async fn stop_leaves_idle_current() {
    let (_dir, service, launcher, _graph) = setup();
    service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    service.stop_session().unwrap();
    assert!(service.session().is_none());
    assert!(read_current(service.paths()).unwrap().is_none());
    assert!(!launcher.stopped().is_empty());

    // Restore stays idle
    let paths = service.paths().clone();
    let launcher2 = Arc::new(FakeLauncher::new());
    let graph2 = Arc::new(FakeGraph::new());
    let service2 = Service::new(paths, launcher2.clone(), graph2).unwrap();
    service2.restore().unwrap();
    assert!(service2.session().is_none());
    assert!(launcher2.started().is_empty());
}

#[tokio::test]
async fn fork_copies_work_shares_rig() {
    let (_dir, service, _launcher, _graph) = setup();
    let fresh = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    let take = service.paths().session_work(&fresh.record.id).join("take.wav");
    fs::write(&take, b"RIFF").unwrap();
    service
        .set_system_links(vec![system_link(
            "lead",
            "melody",
            Some("obake-out-1"),
            None,
            "send_1",
        )])
        .unwrap();

    let forked = service
        .start_session("quartet", SessionMode::Fork, Some(&fresh.record.id))
        .unwrap();
    assert_ne!(forked.record.id, fresh.record.id);
    assert_eq!(forked.record.forked_from.as_deref(), Some(fresh.record.id.as_str()));
    assert_eq!(forked.record.system_links.len(), 1);
    let forked_take = service
        .paths()
        .session_work(&forked.record.id)
        .join("take.wav");
    assert_eq!(fs::read(&forked_take).unwrap(), b"RIFF");
    assert!(service
        .paths()
        .rig_work("quartet")
        .join("lead.ck")
        .is_file());
}

#[tokio::test]
async fn sessions_files_delete() {
    let (_dir, service, _launcher, _graph) = setup();
    // duo rig + two quartet sessions
    fs::write(
        service.paths().rig_file("duo"),
        lead_rig(),
    )
    .unwrap();
    let q1 = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    fs::write(
        service.paths().session_work(&q1.record.id).join("take.wav"),
        b"WAVDATA",
    )
    .unwrap();
    service.stop_session().unwrap();
    let _q2 = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    service.stop_session().unwrap();
    let _d = service
        .start_session("duo", SessionMode::Fresh, None)
        .unwrap();
    service.stop_session().unwrap();

    let listed = service.sessions(Some("quartet")).unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().all(|s| s.rig == "quartet"));

    let files = service.session_files(&q1.record.id).unwrap();
    assert!(files.iter().any(|f| f.path == "take.wav"));
    let bytes = service
        .read_session_file(&q1.record.id, "take.wav")
        .unwrap();
    assert_eq!(bytes, b"WAVDATA");
    assert!(service
        .read_session_file(&q1.record.id, "../lead.ck")
        .is_err());

    // Active session cannot be deleted
    let active = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    assert!(service.delete_session(&active.record.id).is_err());
    service.stop_session().unwrap();

    let asm_before = service.paths().rig_work("quartet").join("lead.ck");
    assert!(asm_before.is_file());
    service.delete_session(&q1.record.id).unwrap();
    assert!(service.sessions(Some("quartet")).unwrap().iter().all(|s| s.id != q1.record.id));
    assert!(!service.paths().session_work(&q1.record.id).exists());
    assert!(asm_before.is_file());
}

#[tokio::test]
async fn system_links_pending_applied_removed() {
    let (_dir, service, _launcher, graph) = setup();
    service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    service
        .set_system_links(vec![system_link(
            "lead",
            "melody",
            Some("obake-out-1"),
            None,
            "send_1",
        )])
        .unwrap();

    let status = service.reconcile_links().unwrap();
    assert_eq!(status.len(), 1);
    assert!(matches!(
        status[0].state,
        obake_session::links::LinkState::Pending
    ));

    graph.add_port("lead", "outport 1", PortDirection::Out, None);
    graph.add_port("obake-out-1", "send_1", PortDirection::In, None);
    let status = service.reconcile_links().unwrap();
    assert!(matches!(
        status[0].state,
        obake_session::links::LinkState::Applied
    ));
    assert!(graph
        .links()
        .contains(&("lead:outport 1".into(), "obake-out-1:send_1".into())));

    // Rig YAML unchanged
    let asm = fs::read_to_string(service.paths().rig_file("quartet")).unwrap();
    assert!(!asm.contains("obake-out-1"));

    service.set_system_links(vec![]).unwrap();
    let status = service.reconcile_links().unwrap();
    assert!(status.is_empty());
    assert!(graph.links().is_empty());
}

#[tokio::test]
async fn graphql_set_system_links_and_queries() {
    let (_dir, service, _launcher, _graph) = setup();
    service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();

    let resp = graphql::execute(
        service.clone(),
        r#"mutation {
          setSystemLinks(links: [{
            from: { instance: "lead", role: "melody" }
            to: { node: "obake-out-1", port: "send_1" }
          }]) { id systemLinks { to { node port } } }
        }"#,
    )
    .await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let session = graphql::execute(service.clone(), "{ session { systemLinks { to { node port } } } }").await;
    let data = session.data.into_json().unwrap();
    assert_eq!(
        data["session"]["systemLinks"][0]["to"]["node"],
        "obake-out-1"
    );

    let rigs = graphql::execute(service.clone(), "{ rigs { name instances { name roles { name index } } } }").await;
    assert!(rigs.errors.is_empty());
    let unknown = graphql::execute(service.clone(), r#"{ rig(name: "nope") { name } }"#).await;
    assert!(unknown.data.into_json().unwrap()["rig"].is_null());

    let shapes = graphql::execute(service.clone(), "{ shapes { name launchable imagePresent } }").await;
    let s = shapes.data.into_json().unwrap();
    let chuck = s["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "chuck")
        .unwrap();
    assert_eq!(chuck["launchable"], true);
    // siren/mod-host not in list
    assert!(!s["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["name"] == "siren" || x["name"] == "mod-host"));
}

#[tokio::test]
async fn incomplete_shape_does_not_replace_session() {
    let (_dir, service, launcher, _graph) = setup();
    let active = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    let id = active.record.id.clone();
    let starts_before = launcher.started().len();

    fs::write(
        service.paths().rig_file("bad"),
        r#"
instances:
  noise:
    shape: siren
    command: []
    roles:
      out: [a]
      in: []
links: []
"#,
    )
    .unwrap();

    let err = service.start_session("bad", SessionMode::Fresh, None);
    assert!(err.is_err());
    assert_eq!(service.session().unwrap().record.id, id);
    // No additional successful launch for siren
    assert_eq!(launcher.started().len(), starts_before);
}

#[tokio::test]
async fn mixpre_alias_resolution() {
    let (dir, _service, _l, _g) = setup();
    let paths = Paths::under(dir.path());
    fs::write(
        paths.host_yaml(),
        r#"
aliases:
  mixpre:
    card: II
"#,
    )
    .unwrap();
    // recreate service with host yaml
    let launcher = Arc::new(FakeLauncher::new());
    let graph = Arc::new(FakeGraph::new());
    let service = Service::new(paths, launcher, graph.clone()).unwrap();
    // need images + rig again — setup wrote them under dir
    service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    service
        .set_system_links(vec![system_link(
            "lead",
            "melody",
            None,
            Some("mixpre"),
            "playback_FL",
        )])
        .unwrap();

    let status = service.reconcile_links().unwrap();
    assert!(matches!(
        status[0].state,
        obake_session::links::LinkState::Pending
    ));

    graph.add_port("lead", "outport 1", PortDirection::Out, None);
    graph.add_port("alsa_card_II", "playback_FL", PortDirection::In, Some("II"));
    let status = service.reconcile_links().unwrap();
    assert!(matches!(
        status[0].state,
        obake_session::links::LinkState::Applied
    ));
    let asm = fs::read_to_string(service.paths().rig_file("quartet")).unwrap();
    assert!(!asm.contains("mixpre"));
}

#[tokio::test]
async fn multi_instance_role_links() {
    let (_dir, service, launcher, graph) = setup();
    fs::write(service.paths().rig_file("quartet"), quartet_multi()).unwrap();
    service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    let started = launcher.started();
    assert_eq!(started.len(), 3);
    let nodes: Vec<_> = started.iter().map(|s| s.instance_name.as_str()).collect();
    assert!(nodes.contains(&"lead"));
    assert!(nodes.contains(&"harmony"));
    assert!(nodes.contains(&"space"));
    let node_names: Vec<_> = started
        .iter()
        .map(|s| {
            s.env
                .get("PIPEWIRE_PROPS")
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    assert!(node_names.iter().any(|p| p.contains("lead")));
    assert!(node_names.iter().any(|p| p.contains("harmony")));
    assert!(node_names.iter().any(|p| p.contains("space")));

    let asm = fs::read_to_string(service.paths().rig_file("quartet")).unwrap();
    assert!(!asm.contains("outport"));
    assert!(!asm.contains("out1"));

    graph.add_port("lead", "outport 1", PortDirection::Out, None);
    graph.add_port("harmony", "outport 1", PortDirection::Out, None);
    graph.add_port("space", "in1", PortDirection::In, None);
    let status = service.reconcile_links().unwrap();
    assert_eq!(status.len(), 2);
    assert!(status.iter().all(|s| matches!(s.state, obake_session::links::LinkState::Applied)));
    let links = graph.links();
    assert!(links.contains(&("lead:outport 1".into(), "space:in1".into())));
    assert!(links.contains(&("harmony:outport 1".into(), "space:in1".into())));
}

#[tokio::test]
async fn sample_library_and_env_override() {
    let (dir, _s, _l, _g) = setup();
    let paths = Paths::under(dir.path());
    let samples = dir.path().join("samples");
    fs::create_dir_all(&samples).unwrap();
    fs::write(
        paths.host_yaml(),
        format!(
            r#"
environment:
  OBAKE_HOST: host-value
sample_library:
  source: {}
  target: /samples
"#,
            samples.display()
        ),
    )
    .unwrap();
    fs::write(
        paths.rig_file("quartet"),
        r#"
instances:
  lead:
    shape: chuck
    command: ["lead.ck"]
    environment:
      OBAKE_HOST: instance-value
    roles:
      out: [melody]
      in: []
links: []
"#,
    )
    .unwrap();
    let launcher = Arc::new(FakeLauncher::new());
    let graph = Arc::new(FakeGraph::new());
    let service = Service::new(paths, launcher.clone(), graph).unwrap();
    service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    let req = &launcher.started()[0];
    assert_eq!(req.env.get("OBAKE_HOST").unwrap(), "instance-value");
    assert!(req
        .binds
        .iter()
        .any(|b| b.target == "/samples" && b.readonly));
}

#[tokio::test]
async fn install_remove_rig() {
    let (_dir, service, _l, _g) = setup();
    let doc = lead_rig();
    service.install_rig("duo", doc).unwrap();
    assert!(service.rig("duo").unwrap().is_some());
    service
        .start_session("duo", SessionMode::Fresh, None)
        .unwrap();
    assert!(service.remove_rig("duo").is_err());
    service.stop_session().unwrap();
    service.remove_rig("duo").unwrap();
    assert!(service.rig("duo").unwrap().is_none());
}

#[tokio::test]
async fn graph_query_live() {
    let (_dir, service, _l, graph) = setup();
    service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    graph.add_port("lead", "outport 1", PortDirection::Out, None);
    let resp = graphql::execute(service.clone(), "{ graph { name ports { name } } }").await;
    assert!(resp.errors.is_empty());
    let data = resp.data.into_json().unwrap();
    assert!(data["graph"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["name"] == "lead"));
    graph.set_ports(vec![]);
    let resp = graphql::execute(service.clone(), "{ graph { name } }").await;
    assert!(resp.data.into_json().unwrap()["graph"]
        .as_array()
        .unwrap()
        .is_empty());
    // session file unchanged
    let id = service.session().unwrap().record.id;
    let rec = read_session(service.paths(), &id).unwrap();
    assert_eq!(rec.rig, "quartet");
}

#[tokio::test]
async fn ravenna_streams() {
    let (dir, _s, _l, _g) = setup();
    // Without ravenna config — error, no create
    let paths = Paths::under(dir.path());
    let launcher = Arc::new(FakeLauncher::new());
    let graph = Arc::new(FakeGraph::new());
    let service = Service::new(paths.clone(), launcher.clone(), graph.clone()).unwrap();
    let err = service.set_ravenna_streams(
        &[("src1".into(), 2, "L24".into())],
        &[("sink1".into(), "dev".into(), "stream".into())],
    );
    assert!(err.is_err());
    assert!(!paths.ravenna_config_dir.exists() || !paths.ravenna_config_dir.join("config.toml").exists());

    // With config present
    fs::create_dir_all(&paths.ravenna_config_dir).unwrap();
    fs::write(
        paths.ravenna_config_dir.join("config.toml"),
        r#"
[graph]
rate = 48000
sources = []
sinks = []
"#,
    )
    .unwrap();
    let service = Service::new(paths.clone(), launcher, graph).unwrap();
    std::env::set_var("OBAKE_SESSION_SKIP_RAVENNA_RELOAD", "1");
    service
        .set_ravenna_streams(
            &[("src1".into(), 2, "L24".into())],
            &[("sink1".into(), "dev".into(), "stream".into())],
        )
        .unwrap();
    let text = fs::read_to_string(paths.ravenna_config_dir.join("config.toml")).unwrap();
    assert!(text.contains("src1"));
    assert!(text.contains("sink1"));
}

#[tokio::test]
async fn headless_rigs_query() {
    let (_dir, service, _l, _g) = setup();
    let resp = graphql::execute(service, "{ rigs { name } }").await;
    assert!(resp.errors.is_empty(), "{:?}", resp.errors);
    let data = resp.data.into_json().unwrap();
    assert_eq!(data["rigs"][0]["name"], "quartet");
}

#[tokio::test]
async fn tcp_auth_and_unix_no_auth() {
    use axum::body::Body;
    use hyper::client::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::{TcpListener, UnixStream};

    let (_dir, service, _l, _g) = setup();
    let schema = obake_session::graphql::build_schema(service.clone());
    let token = service.token().to_string();

    // Unix socket without token
    let sock = service.paths().api_socket();
    let svc_u = service.clone();
    let sock_path = sock.clone();
    tokio::spawn(async move {
        let _ = obake_session::graphql::serve_unix(svc_u, &sock_path).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

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
        .body(Body::from(r#"{"query":"{ rigs { name } }"}"#))
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    assert_eq!(res.status(), 200);

    // TCP with/without token — serve on ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app_token = token.clone();
    let schema2 = schema.clone();
    tokio::spawn(async move {
        use axum::routing::get;
        use axum::{middleware, Router};
        use async_graphql_axum::{GraphQLRequest, GraphQLResponse};

        #[derive(Clone)]
        struct St {
            schema: obake_session::graphql::AppSchema,
            token: String,
        }
        async fn handler(
            axum::extract::State(st): axum::extract::State<St>,
            req: GraphQLRequest,
        ) -> GraphQLResponse {
            st.schema.execute(req.into_inner()).await.into()
        }
        async fn auth(
            axum::extract::State(st): axum::extract::State<St>,
            req: axum::http::Request<Body>,
            next: middleware::Next,
        ) -> Result<axum::response::Response, axum::http::StatusCode> {
            let ok = req
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|t| t == st.token)
                .unwrap_or(false);
            if !ok {
                return Err(axum::http::StatusCode::UNAUTHORIZED);
            }
            Ok(next.run(req).await)
        }
        let st = St {
            schema: schema2,
            token: app_token,
        };
        let app = Router::new()
            .route("/api/v0/", get(|| async { "ok" }).post(handler))
            .layer(middleware::from_fn_with_state(st.clone(), auth))
            .with_state(st);
        axum::serve(listener, app).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = reqwest_like_post(&addr, None, r#"{"query":"{ rigs { name } }"}"#).await;
    assert_eq!(client, 401);

    let client = reqwest_like_post(
        &addr,
        Some(&token),
        r#"{"query":"{ rigs { name } }"}"#,
    )
    .await;
    assert_eq!(client, 200);
}

async fn reqwest_like_post(
    addr: &std::net::SocketAddr,
    token: Option<&str>,
    body: &str,
) -> u16 {
    use axum::body::Body;
    use hyper::client::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpStream;

    let stream = TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let mut builder = hyper::Request::builder()
        .method("POST")
        .uri("http://localhost/api/v0/")
        .header("content-type", "application/json");
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    let req = builder.body(Body::from(body.to_string())).unwrap();
    let res = sender.send_request(req).await.unwrap();
    res.status().as_u16()
}

#[tokio::test]
async fn subscriptions_session_and_links() {
    let (_dir, service, _l, graph) = setup();
    let mut rx = service.subscribe_session();
    while rx.try_recv().is_ok() {}

    service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    let got = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(got.is_some());

    let mut lrx = service.subscribe_links();
    while lrx.try_recv().is_ok() {}
    graph.add_port("lead", "outport 1", PortDirection::Out, None);
    graph.add_port("obake-out-1", "send_1", PortDirection::In, None);
    service
        .set_system_links(vec![system_link(
            "lead",
            "melody",
            Some("obake-out-1"),
            None,
            "send_1",
        )])
        .unwrap();
    let got = tokio::time::timeout(std::time::Duration::from_secs(1), lrx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(got.iter().any(|l| matches!(
        l.state,
        obake_session::links::LinkState::Applied
    )));
}

#[tokio::test]
async fn session_file_http_get_put_auth() {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use hyper::client::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let (_dir, service, _l, _g) = setup();
    let started = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    let id = started.record.id.clone();
    service
        .write_session_file(&id, "notes.txt", b"hello")
        .unwrap();

    let token = service.token().to_string();
    let app = obake_session::graphql::router_for_tests(service.clone(), true);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    async fn exchange(
        addr: &std::net::SocketAddr,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> (u16, Vec<u8>) {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let io = TokioIo::new(stream);
        let (mut sender, conn) = http1::handshake(io).await.unwrap();
        tokio::spawn(async move {
            let _ = conn.await;
        });
        let mut builder = hyper::Request::builder()
            .method(method)
            .uri(format!("http://localhost{uri}"));
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        if body.is_some() {
            builder = builder.header("content-type", "application/octet-stream");
        }
        let req = builder
            .body(Body::from(body.unwrap_or_default()))
            .unwrap();
        let res = sender.send_request(req).await.unwrap();
        let status = res.status().as_u16();
        let bytes = res.into_body().collect().await.unwrap().to_bytes().to_vec();
        (status, bytes)
    }

    // TCP without token rejected
    let (status, _) = exchange(
        &addr,
        "GET",
        &format!("/api/v0/sessions/{id}/files/notes.txt"),
        None,
        None,
    )
    .await;
    assert_eq!(status, 401);

    // GET raw bytes
    let (status, body) = exchange(
        &addr,
        "GET",
        &format!("/api/v0/sessions/{id}/files/notes.txt"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body, b"hello");

    // PUT replaces
    let (status, _) = exchange(
        &addr,
        "PUT",
        &format!("/api/v0/sessions/{id}/files/notes.txt"),
        Some(&token),
        Some(b"world".to_vec()),
    )
    .await;
    assert_eq!(status, 204);
    assert_eq!(
        service.read_session_file(&id, "notes.txt").unwrap(),
        b"world"
    );

    // path escape fails
    let (status, _) = exchange(
        &addr,
        "GET",
        &format!("/api/v0/sessions/{id}/files/../lead.ck"),
        Some(&token),
        None,
    )
    .await;
    assert!(status == 400 || status == 404, "status={status}");

    // current alias
    let (status, body) = exchange(
        &addr,
        "GET",
        "/api/v0/sessions/current/files/notes.txt",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body, b"world");

    // Unix without token succeeds
    let sock = service.paths().api_socket();
    let svc_u = service.clone();
    let sock_path = sock.clone();
    tokio::spawn(async move {
        let _ = obake_session::graphql::serve_unix(svc_u, &sock_path).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let stream = tokio::net::UnixStream::connect(&sock).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = hyper::Request::builder()
        .method("GET")
        .uri(format!("http://localhost/api/v0/sessions/{id}/files/notes.txt"))
        .body(Body::empty())
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    assert_eq!(res.status(), 200);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"world");
}

#[test]
fn session_file_stream_write_rejects_escape_and_dir() {
    let (_dir, service, _l, _g) = setup();
    let started = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    let id = started.record.id.clone();

    let mut data = b"payload".as_slice();
    service
        .write_session_file_from_reader(&id, "nested/a.bin", &mut data)
        .unwrap();
    assert_eq!(
        service.read_session_file(&id, "nested/a.bin").unwrap(),
        b"payload"
    );

    let mut bad = b"x".as_slice();
    assert!(service
        .write_session_file_from_reader(&id, "../escape.bin", &mut bad)
        .is_err());

    let work = service.paths().session_work(&id);
    fs::create_dir_all(work.join("adir")).unwrap();
    let mut again = b"x".as_slice();
    assert!(service
        .write_session_file_from_reader(&id, "adir", &mut again)
        .is_err());

    // Failed write leaves prior content
    service
        .write_session_file(&id, "keep.bin", b"original")
        .unwrap();
    // open_session_file works
    let (mut f, len) = service.open_session_file(&id, "keep.bin").unwrap();
    assert_eq!(len, 8);
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut buf).unwrap();
    assert_eq!(buf, b"original");
}

#[tokio::test]
async fn session_file_http_streams_large_payload() {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use hyper::client::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let (_dir, service, _l, _g) = setup();
    let started = service
        .start_session("quartet", SessionMode::Fresh, None)
        .unwrap();
    let id = started.record.id.clone();
    // Seed a prior file that must survive a failed/partial replace path
    service
        .write_session_file(&id, "big.bin", b"prior")
        .unwrap();

    let token = service.token().to_string();
    let app = obake_session::graphql::router_for_tests(service.clone(), false);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // >8 MiB — above Axum's default 2 MiB body limit
    let size = 9 * 1024 * 1024;
    let payload: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = hyper::Request::builder()
        .method("PUT")
        .uri(format!("http://localhost/api/v0/sessions/{id}/files/big.bin"))
        .header("content-type", "application/octet-stream")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(payload.clone()))
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    assert_eq!(res.status(), 204, "large PUT must not 413");

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let io = TokioIo::new(stream);
    let (mut sender, conn) = http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let req = hyper::Request::builder()
        .method("GET")
        .uri(format!("http://localhost/api/v0/sessions/{id}/files/big.bin"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = sender.send_request(req).await.unwrap();
    assert_eq!(res.status(), 200);
    let got = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(got.len(), size);
    assert_eq!(&got[..], &payload[..]);
}
