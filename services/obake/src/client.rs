// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! HTTP / Unix GraphQL transport.

use crate::config::Resolved;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::Request;
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use std::path::Path;
use thiserror::Error;
use tokio::net::{TcpStream, UnixStream};

/// Client errors.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Transport or protocol failure.
    #[error("{0}")]
    Message(String),
    /// GraphQL errors in the response.
    #[error("graphql: {0}")]
    GraphQl(String),
}

impl ClientError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

/// Connection endpoint.
#[derive(Debug, Clone)]
pub enum Endpoint {
    /// HTTP(S) URL.
    Http { url: String, token: Option<String> },
    /// Unix domain socket path.
    Unix { path: String },
}

impl Endpoint {
    /// Build from a resolved config.
    pub fn from_resolved(r: &Resolved) -> Self {
        if let Some(rest) = r.endpoint.strip_prefix("unix://") {
            Endpoint::Unix {
                path: rest.to_string(),
            }
        } else {
            Endpoint::Http {
                url: r.endpoint.clone(),
                token: r.token.clone(),
            }
        }
    }
}

/// GraphQL client.
#[derive(Debug, Clone)]
pub struct Client {
    endpoint: Endpoint,
}

impl Client {
    /// Create a client for an endpoint.
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }

    /// From resolved connection settings.
    pub fn from_resolved(r: &Resolved) -> Self {
        Self::new(Endpoint::from_resolved(r))
    }

    /// Execute a GraphQL query/mutation and return the `data` value.
    pub async fn execute(&self, query: &str, variables: Option<Value>) -> Result<Value, ClientError> {
        let body = json!({
            "query": query,
            "variables": variables.unwrap_or(Value::Null),
        });
        let bytes = serde_json::to_vec(&body).map_err(|e| ClientError::msg(e.to_string()))?;
        let response_bytes = self.post_json(bytes).await?;
        let value: Value =
            serde_json::from_slice(&response_bytes).map_err(|e| ClientError::msg(e.to_string()))?;
        if let Some(errors) = value.get("errors") {
            if errors.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
                return Err(ClientError::GraphQl(errors.to_string()));
            }
        }
        Ok(value.get("data").cloned().unwrap_or(Value::Null))
    }

    async fn post_json(&self, body: Vec<u8>) -> Result<Vec<u8>, ClientError> {
        match &self.endpoint {
            Endpoint::Http { url, token } => {
                let stream = connect_http(url).await?;
                let io = TokioIo::new(stream);
                let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                tokio::spawn(async move {
                    let _ = conn.await;
                });
                let authority = authority_of(url)?;
                let mut builder = Request::builder()
                    .method("POST")
                    .uri(url)
                    .header("host", authority)
                    .header("content-type", "application/json");
                if let Some(t) = token {
                    builder = builder.header("authorization", format!("Bearer {t}"));
                }
                let req = builder
                    .body(Full::new(Bytes::from(body)))
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                let res = sender
                    .send_request(req)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                if !res.status().is_success() {
                    return Err(ClientError::msg(format!("http {}", res.status())));
                }
                let collected = res
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                Ok(collected.to_bytes().to_vec())
            }
            Endpoint::Unix { path } => {
                let stream = UnixStream::connect(Path::new(path))
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                let io = TokioIo::new(stream);
                let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                tokio::spawn(async move {
                    let _ = conn.await;
                });
                let req = Request::builder()
                    .method("POST")
                    .uri("http://localhost/")
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(body)))
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                let res = sender
                    .send_request(req)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                let collected = res
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                Ok(collected.to_bytes().to_vec())
            }
        }
    }

    /// `{ rigs { name } }`
    pub async fn rigs(&self) -> Result<Value, ClientError> {
        self.execute("{ rigs { name instances { name shape nodeName command roles { name direction index } } links { from { instance role } to { instance role } } } }", None).await
    }

    /// One rig.
    pub async fn rig(&self, name: &str) -> Result<Value, ClientError> {
        self.execute(
            "query($n: String!) { rig(name: $n) { name instances { name shape } } }",
            Some(json!({ "n": name })),
        )
        .await
    }

    /// Shapes query.
    pub async fn shapes(&self) -> Result<Value, ClientError> {
        self.execute("{ shapes { name launchable imagePresent } }", None)
            .await
    }

    /// Active session.
    pub async fn session(&self) -> Result<Value, ClientError> {
        self.execute(
            "{ session { id rig mode phase systemLinks { from { instance role } to { node alias port } } linkStatus { from to state } } }",
            None,
        )
        .await
    }

    /// Recorded sessions.
    pub async fn sessions(&self, rig: Option<&str>) -> Result<Value, ClientError> {
        self.execute(
            "query($a: String) { sessions(rig: $a) { id rig mode forkedFrom } }",
            Some(json!({ "a": rig })),
        )
        .await
    }

    /// Start session.
    pub async fn start(
        &self,
        rig: &str,
        mode: &str,
        fork_from: Option<&str>,
    ) -> Result<Value, ClientError> {
        self.execute(
            "mutation($i: StartSessionInput!) { startSession(input: $i) { id rig mode } }",
            Some(json!({
                "i": {
                    "rig": rig,
                    "mode": mode,
                    "forkFrom": fork_from,
                }
            })),
        )
        .await
    }

    /// Stop session.
    pub async fn stop(&self) -> Result<Value, ClientError> {
        self.execute("mutation { stopSession }", None).await
    }

    /// Set system links (JSON array of link objects).
    pub async fn set_links(&self, links: Value) -> Result<Value, ClientError> {
        self.execute(
            "mutation($l: [SystemLinkInput!]!) { setSystemLinks(links: $l) { id } }",
            Some(json!({ "l": links })),
        )
        .await
    }

    /// List session files.
    pub async fn files(&self, id: &str) -> Result<Value, ClientError> {
        self.execute(
            "query($id: ID!) { sessionFiles(id: $id) { path size directory } }",
            Some(json!({ "id": id })),
        )
        .await
    }

    /// Fetch one session file.
    pub async fn file(&self, id: &str, path: &str) -> Result<Value, ClientError> {
        self.execute(
            "query($id: ID!, $p: String!) { sessionFile(id: $id, path: $p) { path base64 } }",
            Some(json!({ "id": id, "p": path })),
        )
        .await
    }

    /// Delete a session.
    pub async fn delete_session(&self, id: &str) -> Result<Value, ClientError> {
        self.execute(
            "mutation($id: ID!) { deleteSession(id: $id) }",
            Some(json!({ "id": id })),
        )
        .await
    }

    /// Install rig.
    pub async fn install_rig(&self, name: &str, document: &str) -> Result<Value, ClientError> {
        self.execute(
            "mutation($n: String!, $d: String!) { installRig(name: $n, document: $d) { name } }",
            Some(json!({ "n": name, "d": document })),
        )
        .await
    }

    /// Remove rig.
    pub async fn remove_rig(&self, name: &str) -> Result<Value, ClientError> {
        self.execute(
            "mutation($n: String!) { removeRig(name: $n) }",
            Some(json!({ "n": name })),
        )
        .await
    }

    /// Live graph.
    pub async fn graph(&self) -> Result<Value, ClientError> {
        self.execute("{ graph { name ports { name direction } } }", None)
            .await
    }

    /// Set Ravenna streams.
    pub async fn ravenna_set(&self, sources: Value, sinks: Value) -> Result<Value, ClientError> {
        self.execute(
            "mutation($s: [RavennaSourceInput!]!, $k: [RavennaSinkInput!]!) { setRavennaStreams(sources: $s, sinks: $k) }",
            Some(json!({ "s": sources, "k": sinks })),
        )
        .await
    }
}

async fn connect_http(url: &str) -> Result<TcpStream, ClientError> {
    let (host, port) = parse_host_port(url)?;
    TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| ClientError::msg(e.to_string()))
}

fn authority_of(url: &str) -> Result<String, ClientError> {
    let (host, port) = parse_host_port(url)?;
    Ok(format!("{host}:{port}"))
}

fn parse_host_port(url: &str) -> Result<(String, u16), ClientError> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .ok_or_else(|| ClientError::msg(format!("unsupported url: {url}")))?;
    let authority = rest.split('/').next().unwrap_or(rest);
    if let Some((host, port)) = authority.rsplit_once(':') {
        let port: u16 = port
            .parse()
            .map_err(|_| ClientError::msg(format!("bad port in {url}")))?;
        Ok((host.to_string(), port))
    } else {
        let default = if url.starts_with("https://") { 443 } else { 80 };
        Ok((authority.to_string(), default))
    }
}
