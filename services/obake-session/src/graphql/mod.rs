// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! GraphQL schema and Axum server (Unix socket + TCP).

mod schema;

pub use schema::{build_schema, AppSchema};

use crate::service::Service;
use crate::store::ensure_token;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{TcpListener, UnixListener};

#[derive(Clone)]
struct AppState {
    schema: AppSchema,
    token: String,
    require_auth: bool,
}

async fn graphql_handler(
    State(state): State<AppState>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    state.schema.execute(req.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/")))
}

async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if state.require_auth {
        let ok = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|t| t == state.token)
            .unwrap_or(false);
        if !ok {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    Ok(next.run(req).await)
}

fn app_router(schema: AppSchema, token: String, require_auth: bool) -> Router {
    let state = AppState {
        schema: schema.clone(),
        token,
        require_auth,
    };
    Router::new()
        .route("/", get(graphql_playground).post(graphql_handler))
        .route_service("/ws", GraphQLSubscription::new(schema))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

/// Serve GraphQL on a Unix socket (no auth).
pub async fn serve_unix(
    service: Arc<Service>,
    socket_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let schema = build_schema(service.clone());
    let token = service.token().to_string();
    let app = app_router(schema, token, false);
    let listener = UnixListener::bind(socket_path)?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Serve GraphQL on TCP with bearer auth.
pub async fn serve_tcp(
    service: Arc<Service>,
    listen: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let schema = build_schema(service.clone());
    let token = ensure_token(service.paths()).map_err(|e| e.to_string())?;
    let app = app_router(schema, token, true);
    let addr: SocketAddr = listen.parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
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
