// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Rig document parsing and validation.

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors loading or validating a rig.
#[derive(Debug, Error)]
pub enum RigError {
    /// I/O failure.
    #[error("rig i/o: {0}")]
    Io(#[from] std::io::Error),
    /// YAML parse failure.
    #[error("rig parse: {0}")]
    Yaml(#[from] serde_yaml::Error),
    /// Invalid instance key.
    #[error("invalid instance key `{0}`: must not contain ':' or '/'")]
    UnsafeName(String),
    /// Duplicate instance key.
    #[error("duplicate instance key `{0}`")]
    DuplicateName(String),
    /// Malformed role address.
    #[error("invalid role address `{0}`: expected instance.role")]
    BadRoleRef(String),
    /// Link names an unknown instance or role.
    #[error("link refers to unknown endpoint `{0}`")]
    UnknownEndpoint(String),
    /// Both file and directory package forms exist for one name.
    #[error("rig `{0}` has both `<name>.yaml` and `<name>/rig.yaml`")]
    NameConflict(String),
}

/// Port direction for a role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleDirection {
    /// Input role.
    In,
    /// Output role.
    Out,
}

/// One named role on an instance (1-based index within its direction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// Role name.
    pub name: String,
    /// Direction.
    pub direction: RoleDirection,
    /// 1-based slot within that direction.
    pub index: u32,
}

/// One shape instance in a rig.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// Instance key (default node name).
    pub name: String,
    /// Shape profile name.
    pub shape: String,
    /// Effective PipeWire node name.
    pub node_name: String,
    /// Command arguments passed into the shape.
    pub command: Vec<String>,
    /// Ordered roles (outputs then inputs as declared).
    pub roles: Vec<Role>,
    /// Optional per-instance environment overrides.
    pub environment: HashMap<String, String>,
}

/// `instance.role` address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleRef {
    /// Instance key.
    pub instance: String,
    /// Role name.
    pub role: String,
}

impl RoleRef {
    /// Parse `instance.role`.
    pub fn parse(s: &str) -> Result<Self, RigError> {
        let (instance, role) = s
            .split_once('.')
            .ok_or_else(|| RigError::BadRoleRef(s.into()))?;
        if instance.is_empty() || role.is_empty() || role.contains('.') {
            return Err(RigError::BadRoleRef(s.into()));
        }
        Ok(Self {
            instance: instance.into(),
            role: role.into(),
        })
    }

    /// Format as `instance.role`.
    pub fn display(&self) -> String {
        format!("{}.{}", self.instance, self.role)
    }
}

/// Rig role link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigLink {
    /// Source role.
    pub from: RoleRef,
    /// Destination role.
    pub to: RoleRef,
}

/// Parsed rig document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rig {
    /// File stem / GraphQL name.
    pub name: String,
    /// Instances keyed by name.
    pub instances: HashMap<String, Instance>,
    /// Role links.
    pub links: Vec<RigLink>,
}

#[derive(Debug, Default, Deserialize)]
struct RawRoles {
    #[serde(default)]
    out: Vec<String>,
    #[serde(default)]
    #[serde(rename = "in")]
    inputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawInstance {
    shape: String,
    #[serde(default)]
    command: Vec<String>,
    #[serde(default)]
    roles: RawRoles,
    #[serde(default)]
    node: Option<String>,
    #[serde(default)]
    environment: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RawLink {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct RawRig {
    #[serde(deserialize_with = "deserialize_instances")]
    instances: HashMap<String, RawInstance>,
    #[serde(default)]
    links: Vec<RawLink>,
}

fn deserialize_instances<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, RawInstance>, D::Error>
where
    D: Deserializer<'de>,
{
    struct InstancesVisitor;

    impl<'de> Visitor<'de> for InstancesVisitor {
        type Value = HashMap<String, RawInstance>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("instance map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut out = HashMap::new();
            while let Some((key, value)) = map.next_entry::<String, RawInstance>()? {
                if key.contains(':') || key.contains('/') {
                    return Err(de::Error::custom(format!(
                        "invalid instance key `{key}`: must not contain ':' or '/'"
                    )));
                }
                if out.contains_key(&key) {
                    return Err(de::Error::custom(format!(
                        "duplicate instance key `{key}`"
                    )));
                }
                out.insert(key, value);
            }
            Ok(out)
        }
    }

    deserializer.deserialize_map(InstancesVisitor)
}

fn validate_instance_name(name: &str) -> Result<(), RigError> {
    if name.contains(':') || name.contains('/') {
        return Err(RigError::UnsafeName(name.into()));
    }
    Ok(())
}

/// Detect duplicate keys under `instances:` that libyaml may collapse.
fn scan_duplicate_instance_keys(text: &str) -> Result<(), RigError> {
    let mut in_instances = false;
    let mut seen = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && trimmed.starts_with("instances:") {
            in_instances = true;
            continue;
        }
        if in_instances {
            if indent == 0 && trimmed.ends_with(':') && !trimmed.starts_with(' ') {
                break;
            }
            if indent == 2 && trimmed.contains(':') {
                let key = trimmed.split(':').next().unwrap().trim();
                if key.is_empty() {
                    continue;
                }
                if seen.iter().any(|s: &String| s == key) {
                    return Err(RigError::DuplicateName(key.into()));
                }
                seen.push(key.into());
            }
        }
    }
    Ok(())
}

/// Parse a rig document with the given name.
pub fn parse_rig(name: &str, text: &str) -> Result<Rig, RigError> {
    scan_duplicate_instance_keys(text)?;
    let raw: RawRig = serde_yaml::from_str(text)?;
    let mut instances = HashMap::new();
    for (key, raw_inst) in raw.instances {
        validate_instance_name(&key)?;
        let mut roles = Vec::new();
        for (i, role_name) in raw_inst.roles.out.iter().enumerate() {
            roles.push(Role {
                name: role_name.clone(),
                direction: RoleDirection::Out,
                index: (i as u32) + 1,
            });
        }
        for (i, role_name) in raw_inst.roles.inputs.iter().enumerate() {
            roles.push(Role {
                name: role_name.clone(),
                direction: RoleDirection::In,
                index: (i as u32) + 1,
            });
        }
        let node_name = raw_inst.node.unwrap_or_else(|| key.clone());
        validate_instance_name(&node_name)?;
        instances.insert(
            key.clone(),
            Instance {
                name: key,
                shape: raw_inst.shape,
                node_name,
                command: raw_inst.command,
                roles,
                environment: raw_inst.environment,
            },
        );
    }

    let mut links = Vec::new();
    for link in raw.links {
        let from = RoleRef::parse(&link.from)?;
        let to = RoleRef::parse(&link.to)?;
        validate_role_exists(&instances, &from)?;
        validate_role_exists(&instances, &to)?;
        links.push(RigLink { from, to });
    }

    Ok(Rig {
        name: name.into(),
        instances,
        links,
    })
}

fn validate_role_exists(
    instances: &HashMap<String, Instance>,
    role_ref: &RoleRef,
) -> Result<(), RigError> {
    let inst = instances
        .get(&role_ref.instance)
        .ok_or_else(|| RigError::UnknownEndpoint(role_ref.display()))?;
    if !inst.roles.iter().any(|r| r.name == role_ref.role) {
        return Err(RigError::UnknownEndpoint(role_ref.display()));
    }
    Ok(())
}

/// Load one rig YAML by path and name.
pub fn load_rig(name: &str, path: &Path) -> Result<Rig, RigError> {
    let text = fs::read_to_string(path)?;
    parse_rig(name, &text)
}

/// Load every installed package under the rigs directory.
///
/// Accepts `*.yaml` files and subdirectories that contain `rig.yaml`.
/// Fails closed when both forms exist for the same name.
pub fn load_rigs(dir: &Path) -> Result<Vec<Rig>, RigError> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut names: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            if !seen.insert(name.clone()) {
                return Err(RigError::NameConflict(name));
            }
            names.push(name);
            continue;
        }
        if path.is_dir() {
            let def = path.join("rig.yaml");
            if !def.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            if !seen.insert(name.clone()) {
                return Err(RigError::NameConflict(name));
            }
            names.push(name);
        }
    }
    names.sort();
    for name in names {
        let file = dir.join(format!("{name}.yaml"));
        let def = dir.join(&name).join("rig.yaml");
        match (file.is_file(), def.is_file()) {
            (true, true) => return Err(RigError::NameConflict(name)),
            (true, false) => out.push(load_rig(&name, &file)?),
            (false, true) => out.push(load_rig(&name, &def)?),
            (false, false) => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEAD: &str = r#"
instances:
  lead:
    shape: chuck
    command: ["--out:2", "lead.ck"]
    roles:
      out: [melody, bed]
      in: []
links: []
"#;

    #[test]
    fn accepts_lead_example() {
        let a = parse_rig("quartet", LEAD).unwrap();
        let lead = a.instances.get("lead").unwrap();
        assert_eq!(lead.shape, "chuck");
        assert_eq!(lead.node_name, "lead");
        assert_eq!(lead.roles[0].name, "melody");
        assert_eq!(lead.roles[0].index, 1);
        assert_eq!(lead.roles[1].name, "bed");
        assert_eq!(lead.roles[1].index, 2);
    }

    #[test]
    fn load_rigs_file_and_dir_and_conflict() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("duo.yaml"), LEAD).unwrap();
        fs::create_dir_all(root.join("tone")).unwrap();
        fs::write(root.join("tone/rig.yaml"), LEAD).unwrap();
        let loaded = load_rigs(root).unwrap();
        let names: Vec<_> = loaded.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["duo", "tone"]);

        fs::write(root.join("tone.yaml"), LEAD).unwrap();
        let err = load_rigs(root).unwrap_err();
        assert!(matches!(err, RigError::NameConflict(_)));
    }

    #[test]
    fn rejects_colon_in_name() {
        let yaml = r#"
instances:
  "lead:1":
    shape: chuck
    command: []
    roles:
      out: []
      in: []
"#;
        let err = parse_rig("bad", yaml).unwrap_err();
        assert!(matches!(err, RigError::UnsafeName(_)) || err.to_string().contains(':'));
    }

    #[test]
    fn rejects_slash_in_name() {
        let yaml = r#"
instances:
  "lead/1":
    shape: chuck
    command: []
    roles:
      out: []
      in: []
"#;
        let err = parse_rig("bad", yaml).unwrap_err();
        assert!(
            matches!(err, RigError::UnsafeName(_)) || err.to_string().contains('/')
        );
    }

    #[test]
    fn rejects_duplicate_key() {
        let yaml = r#"
instances:
  lead:
    shape: chuck
    command: []
    roles:
      out: [a]
      in: []
  lead:
    shape: chuck
    command: []
    roles:
      out: [b]
      in: []
"#;
        let err = parse_rig("bad", yaml).unwrap_err();
        assert!(matches!(err, RigError::DuplicateName(_)));
    }

    #[test]
    fn link_uses_roles_not_ports() {
        let yaml = r#"
instances:
  lead:
    shape: chuck
    command: []
    roles:
      out: [melody]
      in: []
  space:
    shape: chuck
    command: []
    roles:
      out: []
      in: [wet]
links:
  - from: lead.melody
    to: space.wet
"#;
        let a = parse_rig("duo", yaml).unwrap();
        assert_eq!(a.links[0].from.display(), "lead.melody");
        assert_eq!(a.links[0].to.display(), "space.wet");
    }
}
