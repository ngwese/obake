// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! ZMAN-class HTTP listener: mock advanced page and Bayeux handshake.
//!
//! Service channels that list streams are intentionally absent until a
//! capture names them.

use log::{debug, info, warn};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Listen port for `/cometd/handshake`. The subtype stays `_merging_zman`.
pub const ZMAN_PORT: u16 = 9090;

const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MAX_HTTP_HEAD: usize = 16 * 1024;
const MAX_FRAME: usize = 64 * 1024;

static CLIENT_SEQ: AtomicU64 = AtomicU64::new(1);

/// HTML body for `GET /advanced/index.html`.
pub fn advanced_page(hostname: &str) -> String {
    format!(
        "<!DOCTYPE html>\n\
         <html><head><meta charset=\"utf-8\"><title>obake ZMAN</title></head>\n\
         <body><h1>obake ZMAN emulation</h1>\n\
         <p>Host: {host}</p>\n\
         <p>CometD handshake: /cometd/handshake</p>\n\
         </body></html>\n",
        host = html_escape(hostname)
    )
}

/// Bind `ip:port`. The returned listener is nonblocking and owned by tokio.
pub fn bind(ip: Ipv4Addr, port: u16) -> io::Result<TcpListener> {
    let addr = SocketAddr::from((ip, port));
    let listener = std::net::TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    TcpListener::from_std(listener)
}

/// Accept connections until the process exits.
pub async fn run(listener: TcpListener, hostname: String) {
    let local = listener.local_addr().ok();
    info!("zman: listening on {local:?} host={hostname}");
    loop {
        match listener.accept().await {
            Ok((sock, peer)) => {
                debug!("zman: connection from {peer}");
                let host = hostname.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle(sock, &host).await {
                        debug!("zman: connection ended: {e}");
                    }
                });
            }
            Err(e) => {
                warn!("zman: accept failed: {e}");
                break;
            }
        }
    }
}

async fn handle(mut sock: TcpStream, hostname: &str) -> Result<(), String> {
    let (head, leftover) = read_head(&mut sock).await?;
    let path = request_path(&head);
    if head.starts_with("GET ") && path == "/advanced/index.html" {
        let body = advanced_page(hostname);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        sock.write_all(resp.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if head.starts_with("GET ") && path == "/cometd/handshake" && is_websocket(&head) {
        let key = header_value(&head, "sec-websocket-key")
            .ok_or_else(|| "websocket upgrade missing key".to_string())?;
        let accept = ws_accept(key.trim());
        let resp = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        sock.write_all(resp.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        return ws_session(&mut sock, leftover).await;
    }
    let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    sock.write_all(resp.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn read_head(sock: &mut TcpStream) -> Result<(String, Vec<u8>), String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        if buf.len() > MAX_HTTP_HEAD {
            return Err("http head too large".into());
        }
        let n = sock.read(&mut tmp).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("connection closed before http head".into());
        }
        buf.extend_from_slice(&tmp[..n]);
    };
    let rest = buf.split_off(end);
    let head = String::from_utf8(buf).map_err(|e| e.to_string())?;
    Ok((head, rest))
}

fn request_path(head: &str) -> &str {
    let line = head.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let _method = parts.next();
    let target = parts.next().unwrap_or("/");
    target.split('?').next().unwrap_or(target)
}

fn is_websocket(head: &str) -> bool {
    header_value(head, "upgrade")
        .map(|v| v.to_ascii_lowercase().contains("websocket"))
        .unwrap_or(false)
}

fn header_value<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    for line in head.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        if k.eq_ignore_ascii_case(name) {
            return Some(v.trim());
        }
    }
    None
}

fn ws_accept(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    b64(&hasher.finalize())
}

/// Reply to Bayeux `/meta/*` messages. Unknown channels are rejected and
/// carry no stream list.
pub fn bayeux_reply(text: &str) -> Result<String, String> {
    let msgs: Vec<Value> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(msgs.len());
    for msg in msgs {
        let channel = msg
            .get("channel")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let reply = match channel.as_str() {
            "/meta/handshake" => json!({
                "channel": "/meta/handshake",
                "version": "1.0",
                "minimumVersion": "1.0",
                "supportedConnectionTypes": ["websocket"],
                "clientId": new_client_id(),
                "successful": true,
                "id": id,
            }),
            "/meta/connect" => json!({
                "channel": "/meta/connect",
                "successful": true,
                "clientId": msg.get("clientId").cloned().unwrap_or(Value::String(new_client_id())),
                "advice": {"reconnect": "retry", "interval": 0, "timeout": 20000},
                "id": id,
            }),
            "/meta/disconnect" => json!({
                "channel": "/meta/disconnect",
                "successful": true,
                "id": id,
            }),
            _ => json!({
                "channel": channel,
                "successful": false,
                "error": "unknown channel",
                "id": id,
            }),
        };
        out.push(reply);
    }
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

fn new_client_id() -> String {
    format!("{:016x}", CLIENT_SEQ.fetch_add(1, Ordering::Relaxed))
}

async fn ws_session(sock: &mut TcpStream, leftover: Vec<u8>) -> Result<(), String> {
    let mut buf = leftover;
    let mut tmp = [0u8; 4096];
    loop {
        while let Some(frame) = take_frame(&mut buf)? {
            match frame.opcode {
                0x1 => {
                    let text = String::from_utf8(frame.payload).map_err(|e| e.to_string())?;
                    debug!("zman: bayeux request {text}");
                    let reply = bayeux_reply(&text)?;
                    write_text(sock, &reply).await?;
                }
                0x8 => {
                    write_frame(sock, 0x8, &[]).await?;
                    return Ok(());
                }
                0x9 => write_frame(sock, 0xA, &frame.payload).await?,
                0xA => {}
                other => return Err(format!("unsupported websocket opcode {other}")),
            }
        }
        let n = sock.read(&mut tmp).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(());
        }
        if buf.len() + n > MAX_FRAME * 2 {
            return Err("websocket buffer overflow".into());
        }
        buf.extend_from_slice(&tmp[..n]);
    }
}

struct WsFrame {
    opcode: u8,
    payload: Vec<u8>,
}

fn take_frame(buf: &mut Vec<u8>) -> Result<Option<WsFrame>, String> {
    if buf.len() < 2 {
        return Ok(None);
    }
    let opcode = buf[0] & 0x0f;
    let masked = buf[1] & 0x80 != 0;
    let mut len = (buf[1] & 0x7f) as usize;
    let mut offset = 2;
    if len == 126 {
        if buf.len() < 4 {
            return Ok(None);
        }
        len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
        offset = 4;
    } else if len == 127 {
        return Err("websocket frame too large".into());
    }
    if len > MAX_FRAME {
        return Err("websocket payload too large".into());
    }
    let mask_len = if masked { 4 } else { 0 };
    if buf.len() < offset + mask_len + len {
        return Ok(None);
    }
    let mut payload = buf[offset + mask_len..offset + mask_len + len].to_vec();
    if masked {
        let mask = &buf[offset..offset + 4];
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
    }
    buf.drain(..offset + mask_len + len);
    Ok(Some(WsFrame { opcode, payload }))
}

async fn write_text(sock: &mut TcpStream, text: &str) -> Result<(), String> {
    write_frame(sock, 0x1, text.as_bytes()).await
}

async fn write_frame(sock: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<(), String> {
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.push(0x80 | opcode);
    if payload.len() < 126 {
        out.push(payload.len() as u8);
    } else {
        out.push(126);
        out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    }
    out.extend_from_slice(payload);
    sock.write_all(&out).await.map_err(|e| e.to_string())
}

fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | bytes[i + 2] as u32;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    if i < bytes.len() {
        let rem = bytes.len() - i;
        let mut n = (bytes[i] as u32) << 16;
        if rem == 2 {
            n |= (bytes[i + 1] as u32) << 8;
        }
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if rem == 2 {
            out.push(T[((n >> 6) & 63) as usize] as char);
            out.push('=');
        } else {
            out.push('=');
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn page_names_the_host() {
        let page = advanced_page("bata");
        assert!(page.contains("Host: bata"));
        assert!(page.contains("/cometd/handshake"));
        assert!(page.starts_with("<!DOCTYPE html>"));
    }

    #[tokio::test]
    async fn http_page_and_bayeux_handshake() {
        let listener = bind(Ipv4Addr::LOCALHOST, 0).unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(run(listener, "bata".into()));

        let mut page = TcpStream::connect(addr).await.unwrap();
        page.write_all(b"GET /advanced/index.html HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut buf = vec![0u8; 2048];
        let n = page.read(&mut buf).await.unwrap();
        let text = String::from_utf8_lossy(&buf[..n]);
        assert!(text.contains("200 OK"));
        assert!(text.contains("Host: bata"));
        assert!(text.contains("text/html"));

        let mut ws = TcpStream::connect(addr).await.unwrap();
        ws.write_all(
            b"GET /cometd/handshake HTTP/1.1\r\nHost: localhost\r\n\
              Upgrade: websocket\r\nConnection: Upgrade\r\n\
              Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
              Sec-WebSocket-Version: 13\r\n\r\n",
        )
        .await
        .unwrap();
        let mut head = vec![0u8; 256];
        let n = ws.read(&mut head).await.unwrap();
        let head = String::from_utf8_lossy(&head[..n]);
        assert!(head.contains("101"), "{head}");
        // RFC 6455 sample accept for key dGhlIHNhbXBsZSBub25jZQ==
        assert!(head.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));

        let handshake = r#"[{"channel":"/meta/handshake","version":"1.0","supportedConnectionTypes":["websocket"],"id":"1"}]"#;
        write_client_text(&mut ws, handshake).await;
        let reply = read_client_text(&mut ws).await;
        let parsed: Value = serde_json::from_str(&reply).unwrap();
        let msg = &parsed[0];
        assert_eq!(msg["channel"], "/meta/handshake");
        assert_eq!(msg["successful"], true);
        assert!(msg["clientId"].as_str().unwrap().len() >= 8);
        let types = msg["supportedConnectionTypes"].as_array().unwrap();
        assert!(types.iter().any(|t| t == "websocket"));

        let client_id = msg["clientId"].as_str().unwrap();
        let connect = format!(
            r#"[{{"channel":"/meta/connect","clientId":"{client_id}","connectionType":"websocket","id":"2"}}]"#
        );
        write_client_text(&mut ws, &connect).await;
        let reply = read_client_text(&mut ws).await;
        let parsed: Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(parsed[0]["channel"], "/meta/connect");
        assert_eq!(parsed[0]["successful"], true);
        assert_eq!(parsed[0]["clientId"], client_id);

        write_client_text(
            &mut ws,
            r#"[{"channel":"/service/sources","id":"3"}]"#,
        )
        .await;
        let reply = read_client_text(&mut ws).await;
        assert!(reply.contains("unknown channel"));
        assert!(!reply.contains("obake-out"));
        assert!(!reply.contains("\"data\""));
    }

    async fn write_client_text(sock: &mut TcpStream, text: &str) {
        let payload = text.as_bytes();
        let mask = [1u8, 2, 3, 4];
        let mut frame = Vec::new();
        frame.push(0x81);
        assert!(payload.len() < 126);
        frame.push(0x80 | payload.len() as u8);
        frame.extend_from_slice(&mask);
        for (i, b) in payload.iter().enumerate() {
            frame.push(b ^ mask[i % 4]);
        }
        sock.write_all(&frame).await.unwrap();
    }

    async fn read_client_text(sock: &mut TcpStream) -> String {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 1024];
        loop {
            if let Some(frame) = take_frame(&mut buf).unwrap() {
                assert_eq!(frame.opcode, 0x1);
                return String::from_utf8(frame.payload).unwrap();
            }
            let n = sock.read(&mut tmp).await.unwrap();
            assert!(n > 0);
            buf.extend_from_slice(&tmp[..n]);
        }
    }
}
