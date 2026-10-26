// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Human-readable CLI output (tables). JSON is handled by the caller.

use serde_json::Value;
use std::fmt::Write as _;

/// Which response shape to render.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    /// `rigs` list.
    Rigs,
    /// One `rig`.
    Rig,
    /// `shapes` list.
    Shapes,
    /// Recorded `sessions`.
    Sessions,
    /// Active `session`.
    Session,
    /// `startSession`.
    Start,
    /// `stopSession`.
    Stop,
    /// `setSystemLinks`.
    Links,
    /// `sessionFiles`.
    Files,
    /// `sessionFile`.
    File,
    /// `deleteSession`.
    DeleteSession,
    /// `installRig`.
    InstallRig,
    /// `removeRig`.
    RemoveRig,
    /// Live `graph`.
    Graph,
    /// `setRavennaStreams`.
    Ravenna,
    /// Stub watch.
    Watch,
}

/// Format GraphQL `data` for the terminal.
pub fn render(kind: Kind, data: &Value) -> String {
    match kind {
        Kind::Rigs => format_rigs(data.get("rigs").unwrap_or(data)),
        Kind::Rig => format_rig(data.get("rig").unwrap_or(data)),
        Kind::Shapes => format_shapes(data.get("shapes").unwrap_or(data)),
        Kind::Sessions => format_sessions(data.get("sessions").unwrap_or(data)),
        Kind::Session => format_session(data.get("session").unwrap_or(data)),
        Kind::Start => format_start(data.get("startSession").unwrap_or(data)),
        Kind::Stop => format_bool("stopped", data.get("stopSession").unwrap_or(data)),
        Kind::Links => format_links_result(data.get("setSystemLinks").unwrap_or(data)),
        Kind::Files => format_files(data.get("sessionFiles").unwrap_or(data)),
        Kind::File => format_file(data.get("sessionFile").unwrap_or(data)),
        Kind::DeleteSession => {
            format_bool("deleted", data.get("deleteSession").unwrap_or(data))
        }
        Kind::InstallRig => format_install(data.get("installRig").unwrap_or(data)),
        Kind::RemoveRig => format_bool("removed", data.get("removeRig").unwrap_or(data)),
        Kind::Graph => format_graph(data.get("graph").unwrap_or(data)),
        Kind::Ravenna => format_bool("ok", data.get("setRavennaStreams").unwrap_or(data)),
        Kind::Watch => "watch: use GraphQL websocket at the session endpoint\n".into(),
    }
}

fn format_rigs(v: &Value) -> String {
    let rows: Vec<Vec<String>> = v
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            let name = str_field(r, "name");
            let instances = r
                .get("instances")
                .and_then(|i| i.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|i| {
                            format!(
                                "{}({})",
                                str_field(i, "name"),
                                str_field(i, "shape")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let links = r
                .get("links")
                .and_then(|l| l.as_array())
                .map(|a| a.len().to_string())
                .unwrap_or_else(|| "0".into());
            vec![name, instances, links]
        })
        .collect();
    table(&["NAME", "INSTANCES", "LINKS"], &rows)
}

fn format_rig(v: &Value) -> String {
    if v.is_null() {
        return "no such rig\n".into();
    }
    let mut out = String::new();
    let _ = writeln!(out, "name: {}", str_field(v, "name"));
    let instances = v.get("instances").and_then(|i| i.as_array());
    if let Some(arr) = instances {
        let rows: Vec<Vec<String>> = arr
            .iter()
            .map(|i| {
                let cmd = i
                    .get("command")
                    .and_then(|c| c.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let roles = i
                    .get("roles")
                    .and_then(|r| r.as_array())
                    .map(|a| {
                        a.iter()
                            .map(|r| {
                                format!(
                                    "{}:{}{}",
                                    str_field(r, "direction").chars().next().unwrap_or('?'),
                                    str_field(r, "index"),
                                    {
                                        let n = str_field(r, "name");
                                        if n.is_empty() {
                                            String::new()
                                        } else {
                                            format!("/{n}")
                                        }
                                    }
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                vec![
                    str_field(i, "name"),
                    str_field(i, "shape"),
                    str_field(i, "nodeName"),
                    cmd,
                    roles,
                ]
            })
            .collect();
        out.push_str(&table(
            &["INSTANCE", "SHAPE", "NODE", "COMMAND", "ROLES"],
            &rows,
        ));
    }
    if let Some(links) = v.get("links").and_then(|l| l.as_array()) {
        if !links.is_empty() {
            let rows: Vec<Vec<String>> = links
                .iter()
                .map(|l| {
                    vec![
                        role_ref(l.get("from")),
                        role_ref(l.get("to")),
                    ]
                })
                .collect();
            out.push('\n');
            out.push_str(&table(&["FROM", "TO"], &rows));
        }
    }
    out
}

fn format_shapes(v: &Value) -> String {
    let rows: Vec<Vec<String>> = v
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| {
            vec![
                str_field(s, "name"),
                yes_no(s.get("launchable")),
                yes_no(s.get("imagePresent")),
            ]
        })
        .collect();
    table(&["NAME", "LAUNCHABLE", "IMAGE"], &rows)
}

fn format_sessions(v: &Value) -> String {
    let rows: Vec<Vec<String>> = v
        .as_array()
        .into_iter()
        .flatten()
        .map(|s| {
            vec![
                str_field(s, "id"),
                str_field(s, "rig"),
                str_field(s, "mode"),
                optional_str(s.get("forkedFrom")),
            ]
        })
        .collect();
    table(&["ID", "RIG", "MODE", "FORKED FROM"], &rows)
}

fn format_session(v: &Value) -> String {
    if v.is_null() {
        return "no active session\n".into();
    }
    let mut out = String::new();
    let _ = writeln!(out, "id:     {}", str_field(v, "id"));
    let _ = writeln!(out, "rig:    {}", str_field(v, "rig"));
    let _ = writeln!(out, "mode:   {}", str_field(v, "mode"));
    let _ = writeln!(out, "phase:  {}", str_field(v, "phase"));
    if let Some(links) = v.get("systemLinks").and_then(|l| l.as_array()) {
        out.push('\n');
        let rows: Vec<Vec<String>> = links
            .iter()
            .map(|l| {
                vec![
                    role_ref(l.get("from")),
                    system_endpoint(l.get("to")),
                ]
            })
            .collect();
        out.push_str("system links:\n");
        out.push_str(&table(&["FROM", "TO"], &rows));
    }
    if let Some(status) = v.get("linkStatus").and_then(|l| l.as_array()) {
        out.push('\n');
        let rows: Vec<Vec<String>> = status
            .iter()
            .map(|l| {
                vec![
                    str_field(l, "from"),
                    str_field(l, "to"),
                    str_field(l, "state"),
                ]
            })
            .collect();
        out.push_str("link status:\n");
        out.push_str(&table(&["FROM", "TO", "STATE"], &rows));
    }
    out
}

fn format_start(v: &Value) -> String {
    format!(
        "started  id={}  rig={}  mode={}\n",
        str_field(v, "id"),
        str_field(v, "rig"),
        str_field(v, "mode")
    )
}

fn format_links_result(v: &Value) -> String {
    format!("system links updated  session={}\n", str_field(v, "id"))
}

fn format_files(v: &Value) -> String {
    let rows: Vec<Vec<String>> = v
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| {
            vec![
                str_field(f, "path"),
                f.get("size")
                    .and_then(|s| s.as_u64().or_else(|| s.as_i64().map(|i| i as u64)))
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| str_field(f, "size")),
                yes_no(f.get("directory")),
            ]
        })
        .collect();
    table(&["PATH", "SIZE", "DIR"], &rows)
}

fn format_file(v: &Value) -> String {
    if v.is_null() {
        return "no such file\n".into();
    }
    let b64 = str_field(v, "base64");
    format!(
        "path:   {}\nbytes:  {} (base64)\n",
        str_field(v, "path"),
        b64.len()
    )
}

fn format_install(v: &Value) -> String {
    format!("installed  {}\n", str_field(v, "name"))
}

fn format_graph(v: &Value) -> String {
    let rows: Vec<Vec<String>> = v
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| {
            let ports = n
                .get("ports")
                .and_then(|p| p.as_array())
                .map(|a| {
                    a.iter()
                        .map(|p| {
                            format!(
                                "{}({})",
                                str_field(p, "name"),
                                str_field(p, "direction")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            vec![str_field(n, "name"), ports]
        })
        .collect();
    table(&["NODE", "PORTS"], &rows)
}

fn format_bool(label: &str, v: &Value) -> String {
    match v {
        Value::Bool(true) | Value::Null => format!("{label}\n"),
        Value::Bool(false) => format!("{label}: false\n"),
        other => format!("{label}: {other}\n"),
    }
}

fn role_ref(v: Option<&Value>) -> String {
    let Some(v) = v else {
        return String::new();
    };
    format!("{}.{}", str_field(v, "instance"), str_field(v, "role"))
}

fn system_endpoint(v: Option<&Value>) -> String {
    let Some(v) = v else {
        return String::new();
    };
    let port = str_field(v, "port");
    if let Some(node) = v.get("node").and_then(|n| n.as_str()).filter(|s| !s.is_empty()) {
        format!("{node}:{port}")
    } else if let Some(alias) = v.get("alias").and_then(|a| a.as_str()).filter(|s| !s.is_empty()) {
        format!("{alias}:{port}")
    } else {
        port
    }
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn optional_str(v: Option<&Value>) -> String {
    match v {
        Some(Value::Null) | None => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn yes_no(v: Option<&Value>) -> String {
    match v.and_then(|x| x.as_bool()) {
        Some(true) => "yes".into(),
        Some(false) => "no".into(),
        None => String::new(),
    }
}

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        let mut out = headers.join("  ");
        out.push('\n');
        out.push_str("(none)\n");
        return out;
    }
    let cols = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(cols) {
            widths[i] = widths[i].max(cell.len());
        }
    }
    let mut out = String::new();
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        let _ = write!(out, "{:<width$}", h, width = widths[i]);
    }
    out.push('\n');
    for row in rows {
        for (i, _) in headers.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            let cell = row.get(i).map(String::as_str).unwrap_or("");
            let _ = write!(out, "{:<width$}", cell, width = widths[i]);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rigs_table_has_headers_and_row() {
        let data = json!({
            "rigs": [{
                "name": "tone",
                "instances": [{"name": "tone", "shape": "chuck"}],
                "links": []
            }]
        });
        let text = render(Kind::Rigs, &data);
        assert!(text.contains("NAME"));
        assert!(text.contains("tone"));
        assert!(text.contains("tone(chuck)"));
    }

    #[test]
    fn null_session() {
        assert_eq!(
            render(Kind::Session, &json!({ "session": null })),
            "no active session\n"
        );
    }
}
