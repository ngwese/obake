// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Browser cookie and short-lived device-code logins.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

/// How a request authenticated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cred {
    /// Unix socket.
    Socket,
    /// Host bearer.
    Bearer,
    /// Browser cookie derived from the bearer.
    Cookie,
}

/// Cookie name set by a successful device login.
pub const COOKIE_NAME: &str = "obake_session";

const COOKIE_LABEL: &[u8] = b"obake-browser-v1";
const MAX_PENDING: usize = 32;
const MAX_FAILURES: u32 = 8;
const FAILURE_WINDOW: Duration = Duration::from_secs(60);
/// Device codes stop being acceptable after this long.
pub const LOGIN_TTL: Duration = Duration::from_secs(10 * 60);

type HmacSha256 = Hmac<Sha256>;

/// Hex HMAC of the fixed browser label under the current bearer.
pub fn browser_cookie(bearer: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(bearer.as_bytes()).expect("hmac accepts any key length");
    mac.update(COOKIE_LABEL);
    let bytes = mac.finalize().into_bytes();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Constant-time string compare. Different lengths do not match.
pub fn ct_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// `Set-Cookie` value for a browser session. The server ignores `Max-Age`.
pub fn set_cookie_header(value: &str) -> String {
    format!(
        "{COOKIE_NAME}={value}; HttpOnly; Secure; SameSite=Lax; Path=/api/v0; Max-Age=315360000"
    )
}

/// Read `obake_session` from a Cookie header.
pub fn cookie_value(header: &str) -> Option<&str> {
    for part in header.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(COOKIE_NAME) {
            return rest.strip_prefix('=').map(str::trim);
        }
    }
    None
}

struct PendingLogin {
    poll_secret: String,
    created: Instant,
    approved: bool,
}

/// In-memory device codes. Lost on process restart.
pub struct LoginBook {
    pending: HashMap<String, PendingLogin>,
    failures: u32,
    window_start: Instant,
}

impl LoginBook {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            failures: 0,
            window_start: Instant::now(),
        }
    }

    pub fn start(&mut self) -> Result<LoginStart, String> {
        self.sweep();
        if self.pending.len() >= MAX_PENDING {
            return Err("too many pending logins".into());
        }
        let user_code = user_code();
        let poll_secret = uuid::Uuid::new_v4().to_string();
        self.pending.insert(
            normalize_code(&user_code),
            PendingLogin {
                poll_secret: poll_secret.clone(),
                created: Instant::now(),
                approved: false,
            },
        );
        Ok(LoginStart {
            user_code,
            poll_secret,
        })
    }

    /// `Some(true)` when this poll should set the cookie.
    pub fn poll(&mut self, poll_secret: &str) -> Result<bool, String> {
        self.sweep();
        let key = self
            .pending
            .iter()
            .find(|(_, p)| p.poll_secret == poll_secret)
            .map(|(k, _)| k.clone());
        let Some(key) = key else {
            return Err("unknown login".into());
        };
        let pending = self.pending.get_mut(&key).expect("key present");
        if !pending.approved {
            return Ok(false);
        }
        self.pending.remove(&key);
        Ok(true)
    }

    pub fn approve(&mut self, code: &str) -> Result<(), String> {
        self.note_window();
        if self.failures >= MAX_FAILURES {
            return Err("too many failed approvals".into());
        }
        self.sweep();
        let key = normalize_code(code);
        match self.pending.get_mut(&key) {
            Some(pending) => {
                pending.approved = true;
                Ok(())
            }
            None => {
                self.failures += 1;
                Err("unknown or expired code".into())
            }
        }
    }

    /// Move every pending login into the past (tests).
    pub fn backdate(&mut self, by: Duration) {
        for pending in self.pending.values_mut() {
            pending.created = pending.created.checked_sub(by).unwrap_or(pending.created);
        }
    }

    fn sweep(&mut self) {
        self.pending
            .retain(|_, p| p.created.elapsed() <= LOGIN_TTL);
    }

    fn note_window(&mut self) {
        if self.window_start.elapsed() > FAILURE_WINDOW {
            self.failures = 0;
            self.window_start = Instant::now();
        }
    }
}

impl Default for LoginBook {
    fn default() -> Self {
        Self::new()
    }
}

/// Values shown to the browser and held for polling.
pub struct LoginStart {
    pub user_code: String,
    pub poll_secret: String,
}

fn user_code() -> String {
    const ALPHA: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let bytes = *uuid::Uuid::new_v4().as_bytes();
    let mut s = String::with_capacity(9);
    for i in 0..8 {
        let ch = ALPHA[(bytes[i] as usize) % ALPHA.len()] as char;
        s.push(ch);
        if i == 3 {
            s.push('-');
        }
    }
    s
}

fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .flat_map(|c| c.to_uppercase())
        .collect()
}

/// Login page. The script polls until the cookie is set, then opens GraphiQL.
pub fn login_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>obake</title></head>
<body>
<p>Approve this code from a machine that already has the obake CLI:</p>
<pre id="code">…</pre>
<p id="status">waiting</p>
<script>
async function main() {
  const start = await fetch("/api/v0/login/start", { method: "POST" });
  const body = await start.json();
  document.getElementById("code").textContent = body.userCode;
  const wait = body.intervalMs || 2000;
  for (;;) {
    const res = await fetch("/api/v0/login/poll", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ pollSecret: body.pollSecret })
    });
    const polled = await res.json();
    if (polled.status === "approved") {
      location.href = "/api/v0/";
      return;
    }
    await new Promise((r) => setTimeout(r, wait));
  }
}
main();
</script>
</body>
</html>
"#
}
