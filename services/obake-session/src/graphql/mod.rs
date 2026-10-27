// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! GraphQL schema and Axum server (Unix socket + TCP).

mod schema;

pub use schema::{build_schema, AppSchema};

use crate::auth::{self, browser_cookie, cookie_value, set_cookie_header, Cred};
use crate::service::Service;
use crate::tls::ensure_host_cert;
use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::body::Body;
use http_body_util::BodyExt;
use axum::extract::{DefaultBodyLimit, Extension, Json, Path, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::serve::Listener;
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use std::io;
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use futures_util::StreamExt;
use log::{debug, trace};
use std::net::SocketAddr;
use std::path::Path as FsPath;
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::{TcpListener, UnixListener};
use tokio_util::io::ReaderStream;

#[derive(Clone)]
struct AppState {
    schema: AppSchema,
    service: Arc<Service>,
    require_auth: bool,
}

async fn graphql_handler(
    State(state): State<AppState>,
    Extension(cred): Extension<Cred>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let request = req.into_inner().data(cred);
    debug!(
        target: "obake_session::http",
        "graphql request query={} variables={}",
        compact_graphql(&request.query),
        request.variables
    );
    if log::log_enabled!(target: "obake_session::http", log::Level::Trace) {
        let payload = serde_json::json!({
            "query": request.query,
            "variables": request.variables,
        });
        trace!(
            target: "obake_session::http",
            "graphql request payload={}",
            payload
        );
    }
    let response = state.schema.execute(request).await;
    if response.errors.is_empty() {
        debug!(target: "obake_session::http", "graphql response ok");
    } else {
        debug!(
            target: "obake_session::http",
            "graphql response errors={:?}",
            response.errors
        );
    }
    if log::log_enabled!(target: "obake_session::http", log::Level::Trace) {
        match serde_json::to_string(&response) {
            Ok(s) => trace!(
                target: "obake_session::http",
                "graphql response payload={}",
                s
            ),
            Err(e) => trace!(
                target: "obake_session::http",
                "graphql response serialize error={}",
                e
            ),
        }
    }
    response.into()
}

/// Versioned HTTP API root (GraphQL, subscriptions, session files).
pub const API_V0: &str = "/api/v0";

async fn graphql_ide() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .endpoint(&format!("{API_V0}/"))
            .subscription_endpoint(&format!("{API_V0}/ws"))
            .finish(),
    )
}

async fn request_log_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let cl = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");
    debug!(
        target: "obake_session::http",
        "request {} {} content_length={}",
        method,
        uri,
        cl
    );
    let res = next.run(req).await;
    debug!(
        target: "obake_session::http",
        "response {} {} -> {}",
        method,
        uri,
        res.status()
    );
    res
}

fn is_public(path: &str) -> bool {
    path == format!("{API_V0}/login")
        || path == format!("{API_V0}/login/start")
        || path == format!("{API_V0}/login/poll")
}

fn request_cred(state: &AppState, req: &Request) -> Option<Cred> {
    if !state.require_auth {
        return Some(Cred::Socket);
    }
    if is_public(req.uri().path()) {
        // Login routes are unauthenticated and must not count as the bearer.
        return Some(Cred::Cookie);
    }
    let bearer = state.service.token();
    let header_ok = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| auth::ct_eq(t, &bearer));
    if header_ok {
        return Some(Cred::Bearer);
    }
    let cookie_ok = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(cookie_value)
        .is_some_and(|c| auth::ct_eq(c, &browser_cookie(&bearer)));
    if cookie_ok {
        return Some(Cred::Cookie);
    }
    None
}

async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(cred) = request_cred(&state, &req) else {
        debug!(
            target: "obake_session::http",
            "auth rejected {} {}",
            req.method(),
            req.uri()
        );
        return Err(StatusCode::UNAUTHORIZED);
    };
    req.extensions_mut().insert(cred);
    Ok(next.run(req).await)
}

fn compact_graphql(query: &str) -> String {
    let one_line: String = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= 240 {
        one_line
    } else {
        let truncated: String = one_line.chars().take(240).collect();
        format!("{truncated}…")
    }
}

fn file_error_status(msg: &str) -> StatusCode {
    if msg.contains("escapes") || msg.contains("invalid") || msg.contains("directory") {
        StatusCode::BAD_REQUEST
    } else if msg.contains("not found") || msg.contains("No such file") {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn get_session_file(
    State(state): State<AppState>,
    Path((session_id, path)): Path<(String, String)>,
) -> Response {
    debug!(
        target: "obake_session::http",
        "file GET session={} path={}",
        session_id,
        path
    );
    let id = match state.service.resolve_session_id(&session_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };
    let (file, len) = match state.service.open_session_file(&id, &path) {
        Ok(v) => v,
        Err(e) => {
            let msg = e.to_string();
            debug!(
                target: "obake_session::http",
                "file GET session={} path={} -> error={}",
                id,
                path,
                msg
            );
            return (file_error_status(&msg), msg).into_response();
        }
    };
    debug!(
        target: "obake_session::http",
        "file GET session={} path={} -> ok content_length={}",
        id,
        path,
        len
    );
    let tokio_file = tokio::fs::File::from_std(file);
    let offset = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let label = format!("session={id} path={path}");
    let stream = ReaderStream::new(tokio_file).map(move |res| {
        if let Ok(ref data) = res {
            if log::log_enabled!(target: "obake_session::http", log::Level::Trace) {
                let off = offset.fetch_add(
                    data.len() as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
                trace!(
                    target: "obake_session::http",
                    "file GET {} response chunk offset={} len={} payload={}",
                    label,
                    off,
                    data.len(),
                    escape_payload(data)
                );
            }
        }
        res
    });
    let body = Body::from_stream(stream);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_LENGTH, len.to_string()),
        ],
        body,
    )
        .into_response()
}

async fn put_session_file(
    State(state): State<AppState>,
    Path((session_id, path)): Path<(String, String)>,
    request: Request,
) -> Response {
    let cl = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");
    debug!(
        target: "obake_session::http",
        "file PUT session={} path={} content_length={}",
        session_id,
        path,
        cl
    );
    let id = match state.service.resolve_session_id(&session_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };
    let dest = match state.service.session_file_path_for_write(&id, &path) {
        Ok(p) => p,
        Err(e) => {
            let msg = e.to_string();
            debug!(
                target: "obake_session::http",
                "file PUT session={} path={} -> error={}",
                id,
                path,
                msg
            );
            return (file_error_status(&msg), msg).into_response();
        }
    };
    match stream_put_to_path(&dest, &format!("session={id} path={path}"), request.into_body())
        .await
    {
        Ok(()) => {
            debug!(
                target: "obake_session::http",
                "file PUT session={} path={} -> ok",
                id,
                path
            );
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            debug!(
                target: "obake_session::http",
                "file PUT session={} path={} -> error={}",
                id,
                path,
                msg
            );
            (file_error_status(&msg), msg).into_response()
        }
    }
}

async fn put_rig_package(
    State(state): State<AppState>,
    Extension(cred): Extension<Cred>,
    Path(name): Path<String>,
    body: Body,
) -> Response {
    if !matches!(cred, Cred::Socket | Cred::Bearer) {
        return (StatusCode::FORBIDDEN, "operator credentials required").into_response();
    }
    if name.is_empty() || name.contains('/') || name.contains(':') {
        return (StatusCode::BAD_REQUEST, "invalid rig name").into_response();
    }
    let collected = match body.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    match state.service.install_rig_tar(&name, &collected) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("escapes") || msg.contains("absolute") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::BAD_REQUEST
            };
            (status, msg).into_response()
        }
    }
}

async fn stream_put_to_path(
    dest: &FsPath,
    label: &str,
    body: Body,
) -> Result<(), std::io::Error> {
    let parent = dest.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent")
    })?;
    tokio::fs::create_dir_all(parent).await?;
    let tmp = parent.join(format!(
        ".{}.tmp",
        dest.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("write")
    ));
    let result = async {
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .await?;
        let mut writer = BufWriter::new(file);
        let mut stream = body.into_data_stream();
        let mut offset = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;
            if log::log_enabled!(target: "obake_session::http", log::Level::Trace) {
                trace!(
                    target: "obake_session::http",
                    "file PUT {} request chunk offset={} len={} payload={}",
                    label,
                    offset,
                    chunk.len(),
                    escape_payload(&chunk)
                );
            }
            offset += chunk.len() as u64;
            writer.write_all(&chunk).await?;
        }
        writer.flush().await?;
        writer.get_ref().sync_all().await?;
        drop(writer);
        tokio::fs::rename(&tmp, dest).await?;
        let dir = tokio::fs::File::open(parent).await?;
        dir.sync_all().await?;
        Ok::<(), std::io::Error>(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&tmp).await;
    }
    result
}

fn escape_payload(data: &[u8]) -> String {
    match std::str::from_utf8(data) {
        Ok(s) => s.to_string(),
        Err(_) => data
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn app_router(schema: AppSchema, service: Arc<Service>, require_auth: bool) -> Router {
    let state = AppState {
        schema: schema.clone(),
        service,
        require_auth,
    };
    let files = Router::new()
        .route(
            &format!("{API_V0}/sessions/{{session_id}}/files/{{*path}}"),
            get(get_session_file).put(put_session_file),
        )
        .route(
            &format!("{API_V0}/rigs/{{name}}/package"),
            put(put_rig_package),
        )
        .layer(DefaultBodyLimit::disable());
    let mut router = Router::new()
        .route(
            &format!("{API_V0}/"),
            get(graphql_ide).post(graphql_handler),
        )
        .route_service(
            &format!("{API_V0}/ws"),
            GraphQLSubscription::new(schema),
        )
        .route(&format!("{API_V0}/login"), get(login_page))
        .route(&format!("{API_V0}/login/start"), post(login_start))
        .route(&format!("{API_V0}/login/poll"), post(login_poll))
        .merge(files);
    if !require_auth {
        router = router.route(&format!("{API_V0}/credential"), get(credential));
    }
    router
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn(request_log_middleware))
        .with_state(state)
}

async fn login_page() -> impl IntoResponse {
    Html(auth::login_html())
}

async fn login_start(State(state): State<AppState>) -> Response {
    match state.service.start_login() {
        Ok(started) => (
            StatusCode::OK,
            Json(json!({
                "userCode": started.user_code,
                "pollSecret": started.poll_secret,
                "intervalMs": 2000,
            })),
        )
            .into_response(),
        Err(e) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct PollBody {
    #[serde(rename = "pollSecret")]
    poll_secret: String,
}

async fn login_poll(State(state): State<AppState>, Json(body): Json<PollBody>) -> Response {
    match state.service.poll_login(&body.poll_secret) {
        Ok(true) => {
            let cookie = set_cookie_header(&browser_cookie(&state.service.token()));
            (
                StatusCode::OK,
                [(header::SET_COOKIE, cookie)],
                Json(json!({"status": "approved"})),
            )
                .into_response()
        }
        Ok(false) => (StatusCode::OK, Json(json!({"status": "pending"}))).into_response(),
        Err(_) => (StatusCode::OK, Json(json!({"status": "unknown"}))).into_response(),
    }
}

async fn credential(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "token": state.service.token(),
        "port": state.service.listen_port(),
        "fingerprint": state.service.fingerprint(),
    }))
}

/// Serve GraphQL and session-file HTTP on a Unix socket (no auth).
pub async fn serve_unix(
    service: Arc<Service>,
    socket_path: &FsPath,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let schema = build_schema(service.clone());
    let app = app_router(schema, service, false);
    let listener = UnixListener::bind(socket_path)?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Serve GraphQL and session-file HTTP on TLS (bearer or browser cookie).
pub async fn serve_tcp(
    service: Arc<Service>,
    listen: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = listen.parse()?;
    let listener = TcpListener::bind(addr).await?;
    serve_tls_on(service, listener).await
}

/// Serve the TLS listener on an already-bound socket (tests bind port 0).
pub async fn serve_tls_on(
    service: Arc<Service>,
    listener: TcpListener,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = listener.local_addr()?.port();
    service.set_listen_port(port);
    let (server_config, _fp) = ensure_host_cert(service.paths()).map_err(|e| e.to_string())?;
    let schema = build_schema(service.clone());
    let app = app_router(schema, service, true);
    let tls = TlsListener {
        inner: listener,
        acceptor: TlsAcceptor::from(server_config),
    };
    axum::serve(tls, app).await?;
    Ok(())
}

struct TlsListener {
    inner: TcpListener,
    acceptor: TlsAcceptor,
}

impl Listener for TlsListener {
    type Io = tokio_rustls::server::TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let Ok((tcp, addr)) = self.inner.accept().await else {
                continue;
            };
            if let Ok(stream) = self.acceptor.accept(tcp).await {
                return (stream, addr);
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.inner.local_addr()
    }
}

/// Run both listeners until cancelled.
pub async fn serve(
    service: Arc<Service>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket = service.paths().api_socket();
    let listen = service.host().listen.clone();
    let svc_unix = service.clone();
    let svc_tcp = service.clone();
    let unix = tokio::spawn(async move {
        if let Err(e) = serve_unix(svc_unix, &socket).await {
            log::error!("unix graphql: {e}");
        }
    });
    let tcp = tokio::spawn(async move {
        if let Err(e) = serve_tcp(svc_tcp, &listen).await {
            log::error!("tcp graphql: {e}");
        }
    });
    tokio::select! {
        _ = unix => {}
        _ = tcp => {}
        _ = tokio::signal::ctrl_c() => {
            log::info!("shutting down");
        }
    }
    Ok(())
}

/// Execute a GraphQL query against an in-process schema (tests).
pub async fn execute(
    service: Arc<Service>,
    query: &str,
) -> async_graphql::Response {
    let schema = build_schema(service);
    schema.execute(query).await
}

/// Execute with variables.
pub async fn execute_with(
    service: Arc<Service>,
    request: async_graphql::Request,
) -> async_graphql::Response {
    let schema = build_schema(service);
    schema.execute(request).await
}

/// Build the Axum router (tests).
pub fn router_for_tests(
    service: Arc<Service>,
    require_auth: bool,
) -> Router {
    let schema = build_schema(service.clone());
    app_router(schema, service, require_auth)
}
