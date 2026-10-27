// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! HTTP / Unix GraphQL transport.

#[path = "pin.rs"]
mod pin;

use crate::config::Resolved;
use bytes::Bytes;
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::Frame;
use hyper::Request;
use hyper_util::rt::TokioIo;
use log::{debug, trace};
use serde_json::{json, Value};
use std::path::Path;
use thiserror::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpStream, UnixStream};
use tokio_rustls::client::TlsStream;
use tokio_util::io::ReaderStream;

/// Read/write chunk size for session-file HTTP (~1 MiB).
pub const FILE_STREAM_CHUNK: usize = 1024 * 1024;

/// Versioned API prefix on the session listener (matches the service).
pub const API_V0: &str = "/api/v0";

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
    Http {
        url: String,
        token: Option<String>,
        fingerprint: Option<String>,
    },
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
                fingerprint: r.fingerprint.clone(),
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
        let vars = variables.clone().unwrap_or(Value::Null);
        debug!(
            target: "obake::client",
            "graphql request endpoint={} query={} variables={}",
            self.endpoint_label(),
            compact_graphql(query),
            vars
        );
        let body = json!({
            "query": query,
            "variables": vars,
        });
        let bytes = serde_json::to_vec(&body).map_err(|e| ClientError::msg(e.to_string()))?;
        trace!(
            target: "obake::client",
            "graphql request payload={}",
            String::from_utf8_lossy(&bytes)
        );
        let response_bytes = self.post_json(bytes).await?;
        trace!(
            target: "obake::client",
            "graphql response payload={}",
            String::from_utf8_lossy(&response_bytes)
        );
        let value: Value =
            serde_json::from_slice(&response_bytes).map_err(|e| ClientError::msg(e.to_string()))?;
        if let Some(errors) = value.get("errors") {
            if errors.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
                debug!(
                    target: "obake::client",
                    "graphql response errors={}",
                    errors
                );
                return Err(ClientError::GraphQl(errors.to_string()));
            }
        }
        debug!(
            target: "obake::client",
            "graphql response ok bytes={}",
            response_bytes.len()
        );
        Ok(value.get("data").cloned().unwrap_or(Value::Null))
    }

    fn endpoint_label(&self) -> String {
        match &self.endpoint {
            Endpoint::Http { url, .. } => url.clone(),
            Endpoint::Unix { path } => format!("unix://{path}"),
        }
    }

    async fn post_json(&self, body: Vec<u8>) -> Result<Vec<u8>, ClientError> {
        match &self.endpoint {
            Endpoint::Http { url, token, fingerprint } => {
                ensure_pin(url, fingerprint)?;
                let gql = join_api_root(url);
                debug!(
                    target: "obake::client",
                    "http POST {} content_length={}",
                    gql,
                    body.len()
                );
                let io = connect_io(url, fingerprint.as_deref()).await?;
                let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                tokio::spawn(async move {
                    let _ = conn.await;
                });
                let authority = authority_of(url)?;
                let mut builder = Request::builder()
                    .method("POST")
                    .uri(&gql)
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
                let status = res.status();
                debug!(target: "obake::client", "http POST {} -> {}", gql, status);
                if !status.is_success() {
                    return Err(ClientError::msg(format!("http {status}")));
                }
                let collected = res
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                Ok(collected.to_bytes().to_vec())
            }
            Endpoint::Unix { path } => {
                let label = format!("unix://{path}{API_V0}/");
                debug!(
                    target: "obake::client",
                    "http POST {} content_length={}",
                    label,
                    body.len()
                );
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
                    .uri(format!("http://localhost{API_V0}/"))
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(body)))
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                let res = sender
                    .send_request(req)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                let status = res.status();
                debug!(target: "obake::client", "http POST {} -> {}", label, status);
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

    /// HTTP GET a session work-area file, writing chunks to `writer`.
    pub async fn get_file_to<W: AsyncWrite + Unpin>(
        &self,
        session_id: &str,
        path: &str,
        writer: &mut W,
    ) -> Result<u64, ClientError> {
        let uri = file_uri(session_id, path);
        debug!(
            target: "obake::client",
            "http GET {}{}",
            self.endpoint_label().trim_end_matches('/'),
            uri
        );
        let n = self.http_stream_get(&uri, writer).await?;
        debug!(
            target: "obake::client",
            "http GET {}{} -> ok bytes={}",
            self.endpoint_label().trim_end_matches('/'),
            uri,
            n
        );
        Ok(n)
    }

    /// HTTP PUT a local file into a session work-area path (streamed).
    pub async fn put_file_from_path(
        &self,
        session_id: &str,
        remote_path: &str,
        local_path: &Path,
    ) -> Result<u64, ClientError> {
        let meta = tokio::fs::metadata(local_path)
            .await
            .map_err(|e| ClientError::msg(e.to_string()))?;
        let len = meta.len();
        let file = tokio::fs::File::open(local_path)
            .await
            .map_err(|e| ClientError::msg(e.to_string()))?;
        let uri = file_uri(session_id, remote_path);
        debug!(
            target: "obake::client",
            "http PUT {}{} content_length={} local={}",
            self.endpoint_label().trim_end_matches('/'),
            uri,
            len,
            local_path.display()
        );
        self.http_stream_put(&uri, file, Some(len)).await?;
        debug!(
            target: "obake::client",
            "http PUT {}{} -> ok bytes={}",
            self.endpoint_label().trim_end_matches('/'),
            uri,
            len
        );
        Ok(len)
    }

    async fn http_stream_get<W: AsyncWrite + Unpin>(
        &self,
        path_and_query: &str,
        writer: &mut W,
    ) -> Result<u64, ClientError> {
        let res = self
            .send_file_request_len(
                "GET",
                path_and_query,
                Full::new(Bytes::new()),
                Some(0),
            )
            .await?;
        let status = res.status();
        if !status.is_success() {
            let collected = res
                .into_body()
                .collect()
                .await
                .map_err(|e| ClientError::msg(e.to_string()))?;
            let bytes = collected.to_bytes();
            let msg = String::from_utf8_lossy(&bytes);
            debug!(
                target: "obake::client",
                "http GET {} -> {} body={}",
                path_and_query,
                status,
                truncate_chars(&msg, 200)
            );
            trace!(
                target: "obake::client",
                "http GET {} -> {} response payload={}",
                path_and_query,
                status,
                msg
            );
            return Err(ClientError::msg(format!("http {status}: {msg}")));
        }
        let mut written = 0u64;
        let mut body = res.into_body();
        while let Some(frame) = body
            .frame()
            .await
            .transpose()
            .map_err(|e| ClientError::msg(e.to_string()))?
        {
            if let Ok(data) = frame.into_data() {
                if log::log_enabled!(target: "obake::client", log::Level::Trace) {
                    trace!(
                        target: "obake::client",
                        "http GET {} response chunk offset={} len={} payload={}",
                        path_and_query,
                        written,
                        data.len(),
                        escape_payload(&data)
                    );
                }
                writer
                    .write_all(&data)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                written += data.len() as u64;
            }
        }
        writer
            .flush()
            .await
            .map_err(|e| ClientError::msg(e.to_string()))?;
        Ok(written)
    }

    async fn http_stream_put(
        &self,
        path_and_query: &str,
        file: tokio::fs::File,
        content_length: Option<u64>,
    ) -> Result<(), ClientError> {
        let reader = ReaderStream::with_capacity(file, FILE_STREAM_CHUNK);
        let offset = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let label = path_and_query.to_string();
        let stream = reader.map_ok(move |chunk| {
            if log::log_enabled!(target: "obake::client", log::Level::Trace) {
                let off = offset.fetch_add(
                    chunk.len() as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
                trace!(
                    target: "obake::client",
                    "http PUT {} request chunk offset={} len={} payload={}",
                    label,
                    off,
                    chunk.len(),
                    escape_payload(&chunk)
                );
            }
            Frame::data(chunk)
        });
        let body = StreamBody::new(stream);
        let res = self
            .send_file_request_len("PUT", path_and_query, body, content_length)
            .await?;
        let status = res.status();
        if !status.is_success() {
            let collected = res
                .into_body()
                .collect()
                .await
                .map_err(|e| ClientError::msg(e.to_string()))?;
            let bytes = collected.to_bytes();
            let msg = String::from_utf8_lossy(&bytes);
            debug!(
                target: "obake::client",
                "http PUT {} -> {} body={}",
                path_and_query,
                status,
                truncate_chars(&msg, 200)
            );
            trace!(
                target: "obake::client",
                "http PUT {} -> {} response payload={}",
                path_and_query,
                status,
                msg
            );
            return Err(ClientError::msg(format!("http {status}: {msg}")));
        }
        Ok(())
    }

    async fn send_file_request_len<B>(
        &self,
        method: &str,
        path_and_query: &str,
        body: B,
        content_length: Option<u64>,
    ) -> Result<hyper::Response<hyper::body::Incoming>, ClientError>
    where
        B: http_body::Body + Send + 'static,
        B::Data: Send + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        match &self.endpoint {
            Endpoint::Http { url, token, fingerprint } => {
                ensure_pin(url, fingerprint)?;
                let io = connect_io(url, fingerprint.as_deref()).await?;
                let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                tokio::spawn(async move {
                    let _ = conn.await;
                });
                let authority = authority_of(url)?;
                let full = format!("{}{path_and_query}", url.trim_end_matches('/'));
                let mut builder = Request::builder()
                    .method(method)
                    .uri(&full)
                    .header("host", authority);
                if method == "PUT" {
                    builder = builder.header("content-type", "application/octet-stream");
                }
                if let Some(n) = content_length {
                    builder = builder.header("content-length", n.to_string());
                }
                if let Some(t) = token {
                    builder = builder.header("authorization", format!("Bearer {t}"));
                }
                let req = builder
                    .body(body)
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                sender
                    .send_request(req)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))
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
                let mut builder = Request::builder()
                    .method(method)
                    .uri(format!("http://localhost{path_and_query}"));
                if method == "PUT" {
                    builder = builder.header("content-type", "application/octet-stream");
                }
                if let Some(n) = content_length {
                    builder = builder.header("content-length", n.to_string());
                }
                let req = builder
                    .body(body)
                    .map_err(|e| ClientError::msg(e.to_string()))?;
                sender
                    .send_request(req)
                    .await
                    .map_err(|e| ClientError::msg(e.to_string()))
            }
        }
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

    /// Approve a browser device code.
    pub async fn approve_login(&self, code: &str) -> Result<(), ClientError> {
        let data = self
            .execute(
                "mutation($c: String!) { approveLogin(code: $c) }",
                Some(json!({ "c": code })),
            )
            .await?;
        if data.get("approveLogin").and_then(|v| v.as_bool()) == Some(true) {
            Ok(())
        } else {
            Err(ClientError::msg(format!("approve failed: {data}")))
        }
    }

    /// Rotate the host bearer. Returns the new bearer.
    pub async fn rotate_auth(&self) -> Result<String, ClientError> {
        let data = self.execute("mutation { rotateAuth }", None).await?;
        data.get("rotateAuth")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| ClientError::msg(format!("rotate failed: {data}")))
    }

    /// GET a path and return the body bytes.
    pub async fn get_raw(&self, path: &str) -> Result<Vec<u8>, ClientError> {
        let res = self
            .send_file_request_len("GET", path, Full::new(Bytes::new()), Some(0))
            .await?;
        let status = res.status();
        let collected = res
            .into_body()
            .collect()
            .await
            .map_err(|e| ClientError::msg(e.to_string()))?;
        if !status.is_success() {
            return Err(ClientError::msg(format!("http {status}")));
        }
        Ok(collected.to_bytes().to_vec())
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

fn compact_graphql(query: &str) -> String {
    let one_line: String = query.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_chars(&one_line, 240)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}…")
    }
}

/// UTF-8 payload when valid; otherwise a hex dump (TRACE file bodies).
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

fn join_api_root(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if base.ends_with(API_V0) {
        format!("{base}/")
    } else {
        format!("{base}{API_V0}/")
    }
}

fn file_uri(session_id: &str, path: &str) -> String {
    format!(
        "{API_V0}/sessions/{}/files/{}",
        encode_segment(session_id),
        encode_rel_path(path)
    )
}

fn encode_rel_path(path: &str) -> String {
    path.split('/')
        .map(encode_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_segment(seg: &str) -> String {
    let mut out = String::new();
    for b in seg.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn ensure_pin(url: &str, fingerprint: &Option<String>) -> Result<(), ClientError> {
    if url.starts_with("https://")
        && fingerprint.as_ref().map(|s| s.is_empty()).unwrap_or(true)
    {
        return Err(ClientError::msg(
            "https endpoint requires a certificate fingerprint",
        ));
    }
    Ok(())
}

enum Conn {
    Tcp(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl AsyncRead for Conn {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Tcp(s) => Pin::new(s).poll_read(cx, buf),
            Conn::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Conn {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Conn::Tcp(s) => Pin::new(s).poll_write(cx, buf),
            Conn::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Tcp(s) => Pin::new(s).poll_flush(cx),
            Conn::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Conn::Tcp(s) => Pin::new(s).poll_shutdown(cx),
            Conn::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

async fn connect_io(url: &str, fingerprint: Option<&str>) -> Result<TokioIo<Conn>, ClientError> {
    let tcp = connect_http(url).await?;
    if url.starts_with("https://") {
        let fp = fingerprint.unwrap_or("");
        let (host, _) = parse_host_port(url)?;
        let config = pin::client_config(fp).map_err(ClientError::msg)?;
        let name = pin::server_name(&host).map_err(ClientError::msg)?;
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let tls = connector
            .connect(name, tcp)
            .await
            .map_err(|e| ClientError::msg(e.to_string()))?;
        Ok(TokioIo::new(Conn::Tls(tls)))
    } else {
        Ok(TokioIo::new(Conn::Tcp(tcp)))
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
