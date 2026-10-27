// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! async-graphql types matching design.md.

use crate::config::RoleDirection as CfgDir;
use crate::links::LinkState as LinkStateEnum;
use crate::service::{ActiveSession, Service, SessionPhase as PhaseEnum};
use crate::store::{
    RoleRefSerde, SessionMode as ModeEnum, SessionRecord, SystemEndpoint, SystemLink,
};
use async_graphql::{
    Context, Enum, InputObject, Object, Result, Schema, SimpleObject, Subscription, ID,
};
use futures_util::stream::Stream;
use std::sync::Arc;

/// Built GraphQL schema type.
pub type AppSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the schema with service in data.
pub fn build_schema(service: Arc<Service>) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(service)
        .finish()
}

fn svc<'a>(ctx: &'a Context<'_>) -> Result<&'a Arc<Service>> {
    ctx.data::<Arc<Service>>()
        .map_err(|_| async_graphql::Error::new("service missing"))
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
enum Direction {
    In,
    Out,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
enum SessionMode {
    Fresh,
    Fork,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
enum SessionPhase {
    Provisioning,
    Linked,
    TearingDown,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
enum LinkState {
    Pending,
    Applied,
}

impl From<CfgDir> for Direction {
    fn from(d: CfgDir) -> Self {
        match d {
            CfgDir::In => Direction::In,
            CfgDir::Out => Direction::Out,
        }
    }
}

impl From<ModeEnum> for SessionMode {
    fn from(m: ModeEnum) -> Self {
        match m {
            ModeEnum::Fresh => SessionMode::Fresh,
            ModeEnum::Fork => SessionMode::Fork,
        }
    }
}

impl From<SessionMode> for ModeEnum {
    fn from(m: SessionMode) -> Self {
        match m {
            SessionMode::Fresh => ModeEnum::Fresh,
            SessionMode::Fork => ModeEnum::Fork,
        }
    }
}

impl From<PhaseEnum> for SessionPhase {
    fn from(p: PhaseEnum) -> Self {
        match p {
            PhaseEnum::Provisioning => SessionPhase::Provisioning,
            PhaseEnum::Linked => SessionPhase::Linked,
            PhaseEnum::TearingDown => SessionPhase::TearingDown,
        }
    }
}

impl From<LinkStateEnum> for LinkState {
    fn from(s: LinkStateEnum) -> Self {
        match s {
            LinkStateEnum::Pending => LinkState::Pending,
            LinkStateEnum::Applied => LinkState::Applied,
        }
    }
}

#[derive(SimpleObject)]
struct Role {
    name: String,
    direction: Direction,
    index: i32,
}

#[derive(SimpleObject)]
struct Instance {
    name: String,
    shape: String,
    #[graphql(name = "nodeName")]
    node_name: String,
    command: Vec<String>,
    roles: Vec<Role>,
}

#[derive(SimpleObject)]
struct RoleRef {
    instance: String,
    role: String,
}

#[derive(SimpleObject)]
struct RigLink {
    from: RoleRef,
    to: RoleRef,
}

#[derive(SimpleObject)]
struct Rig {
    name: String,
    instances: Vec<Instance>,
    links: Vec<RigLink>,
}

#[derive(SimpleObject)]
struct Shape {
    name: String,
    launchable: bool,
    #[graphql(name = "imagePresent")]
    image_present: bool,
}

#[derive(SimpleObject)]
struct SystemEndpointGql {
    alias: Option<String>,
    node: Option<String>,
    port: String,
}

#[derive(SimpleObject)]
struct SystemLinkGql {
    from: RoleRef,
    to: SystemEndpointGql,
}

#[derive(SimpleObject)]
struct LinkStatusGql {
    from: String,
    to: String,
    state: LinkState,
}

#[derive(SimpleObject)]
struct Session {
    id: ID,
    rig: String,
    mode: SessionMode,
    #[graphql(name = "forkedFrom")]
    forked_from: Option<ID>,
    phase: SessionPhase,
    #[graphql(name = "systemLinks")]
    system_links: Vec<SystemLinkGql>,
    #[graphql(name = "linkStatus")]
    link_status: Vec<LinkStatusGql>,
}

#[derive(SimpleObject)]
struct SessionRecordGql {
    id: ID,
    rig: String,
    mode: SessionMode,
    #[graphql(name = "forkedFrom")]
    forked_from: Option<ID>,
}

#[derive(SimpleObject)]
struct SessionFile {
    path: String,
    size: i32,
    directory: bool,
}

#[derive(SimpleObject)]
struct GraphPort {
    name: String,
    direction: Direction,
}

#[derive(SimpleObject)]
struct GraphNode {
    name: String,
    ports: Vec<GraphPort>,
}

#[derive(InputObject)]
struct StartSessionInput {
    rig: String,
    mode: SessionMode,
    #[graphql(name = "forkFrom")]
    fork_from: Option<ID>,
}

#[derive(InputObject)]
struct RoleRefInput {
    instance: String,
    role: String,
}

#[derive(InputObject)]
struct SystemEndpointInput {
    alias: Option<String>,
    node: Option<String>,
    port: String,
}

#[derive(InputObject)]
struct SystemLinkInput {
    from: RoleRefInput,
    to: SystemEndpointInput,
}

#[derive(InputObject)]
struct RavennaSourceInput {
    name: String,
    channels: i32,
    format: String,
}

#[derive(InputObject)]
struct RavennaSinkInput {
    name: String,
    device: String,
    stream: String,
}

fn map_rig(a: crate::config::Rig) -> Rig {
    let mut instances: Vec<_> = a.instances.into_values().collect();
    instances.sort_by(|x, y| x.name.cmp(&y.name));
    Rig {
        name: a.name,
        instances: instances
            .into_iter()
            .map(|i| Instance {
                name: i.name,
                shape: i.shape,
                node_name: i.node_name,
                command: i.command,
                roles: i
                    .roles
                    .into_iter()
                    .map(|r| Role {
                        name: r.name,
                        direction: r.direction.into(),
                        index: r.index as i32,
                    })
                    .collect(),
            })
            .collect(),
        links: a
            .links
            .into_iter()
            .map(|l| RigLink {
                from: RoleRef {
                    instance: l.from.instance,
                    role: l.from.role,
                },
                to: RoleRef {
                    instance: l.to.instance,
                    role: l.to.role,
                },
            })
            .collect(),
    }
}

fn map_session(s: ActiveSession) -> Session {
    Session {
        id: ID(s.record.id.clone()),
        rig: s.record.rig.clone(),
        mode: s.record.mode.into(),
        forked_from: s.record.forked_from.map(ID),
        phase: s.phase.into(),
        system_links: s
            .record
            .system_links
            .iter()
            .map(|l| SystemLinkGql {
                from: RoleRef {
                    instance: l.from.instance.clone(),
                    role: l.from.role.clone(),
                },
                to: SystemEndpointGql {
                    alias: l.to.alias.clone(),
                    node: l.to.node.clone(),
                    port: l.to.port.clone(),
                },
            })
            .collect(),
        link_status: s
            .link_status
            .iter()
            .map(|l| LinkStatusGql {
                from: l.from.clone(),
                to: l.to.clone(),
                state: l.state.into(),
            })
            .collect(),
    }
}

fn map_record(r: SessionRecord) -> SessionRecordGql {
    SessionRecordGql {
        id: ID(r.id),
        rig: r.rig,
        mode: r.mode.into(),
        forked_from: r.forked_from.map(ID),
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn rigs(&self, ctx: &Context<'_>) -> Result<Vec<Rig>> {
        let list = svc(ctx)?.rigs().map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(list.into_iter().map(map_rig).collect())
    }

    async fn rig(&self, ctx: &Context<'_>, name: String) -> Result<Option<Rig>> {
        Ok(svc(ctx)?
            .rig(&name)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .map(map_rig))
    }

    async fn shapes(&self, ctx: &Context<'_>) -> Result<Vec<Shape>> {
        Ok(svc(ctx)?
            .shapes()
            .into_iter()
            .map(|s| Shape {
                name: s.name,
                launchable: s.launchable,
                image_present: s.image_present,
            })
            .collect())
    }

    async fn session(&self, ctx: &Context<'_>) -> Result<Option<Session>> {
        Ok(svc(ctx)?.session().map(map_session))
    }

    async fn sessions(
        &self,
        ctx: &Context<'_>,
        rig: Option<String>,
    ) -> Result<Vec<SessionRecordGql>> {
        let list = svc(ctx)?
            .sessions(rig.as_deref())
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(list.into_iter().map(map_record).collect())
    }

    async fn session_files(&self, ctx: &Context<'_>, id: ID) -> Result<Vec<SessionFile>> {
        let list = svc(ctx)?
            .session_files(id.as_str())
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(list
            .into_iter()
            .map(|f| SessionFile {
                path: f.path,
                size: f.size as i32,
                directory: f.directory,
            })
            .collect())
    }

    async fn graph(&self, ctx: &Context<'_>) -> Result<Vec<GraphNode>> {
        Ok(svc(ctx)?
            .graph()
            .into_iter()
            .map(|(name, ports)| GraphNode {
                name,
                ports: ports
                    .into_iter()
                    .map(|(n, d)| GraphPort {
                        name: n,
                        direction: d.into(),
                    })
                    .collect(),
            })
            .collect())
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn start_session(
        &self,
        ctx: &Context<'_>,
        input: StartSessionInput,
    ) -> Result<Session> {
        let mode: ModeEnum = input.mode.into();
        let fork = input.fork_from.as_ref().map(|id| id.as_str());
        let s = svc(ctx)?
            .start_session(&input.rig, mode, fork)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(map_session(s))
    }

    async fn stop_session(&self, ctx: &Context<'_>) -> Result<bool> {
        svc(ctx)?
            .stop_session()
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    async fn set_system_links(
        &self,
        ctx: &Context<'_>,
        links: Vec<SystemLinkInput>,
    ) -> Result<Session> {
        let converted: Vec<SystemLink> = links
            .into_iter()
            .map(|l| SystemLink {
                from: RoleRefSerde {
                    instance: l.from.instance,
                    role: l.from.role,
                },
                to: SystemEndpoint {
                    alias: l.to.alias,
                    node: l.to.node,
                    port: l.to.port,
                },
            })
            .collect();
        let s = svc(ctx)?
            .set_system_links(converted)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(map_session(s))
    }

    async fn delete_session(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        svc(ctx)?
            .delete_session(id.as_str())
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    async fn install_rig(
        &self,
        ctx: &Context<'_>,
        name: String,
        document: String,
    ) -> Result<Rig> {
        let a = svc(ctx)?
            .install_rig(&name, &document)
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(map_rig(a))
    }

    async fn remove_rig(&self, ctx: &Context<'_>, name: String) -> Result<bool> {
        svc(ctx)?
            .remove_rig(&name)
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    async fn set_ravenna_streams(
        &self,
        ctx: &Context<'_>,
        sources: Vec<RavennaSourceInput>,
        sinks: Vec<RavennaSinkInput>,
    ) -> Result<bool> {
        let sources: Vec<_> = sources
            .into_iter()
            .map(|s| (s.name, s.channels as u32, s.format))
            .collect();
        let sinks: Vec<_> = sinks
            .into_iter()
            .map(|s| (s.name, s.device, s.stream))
            .collect();
        svc(ctx)?
            .set_ravenna_streams(&sources, &sinks)
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn session(
        &self,
        ctx: &Context<'_>,
    ) -> Result<impl Stream<Item = Option<Session>>> {
        let service = svc(ctx)?.clone();
        let mut rx = service.subscribe_session();
        Ok(async_stream::stream! {
            yield service.session().map(map_session);
            loop {
                match rx.recv().await {
                    Ok(s) => yield s.map(map_session),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        })
    }

    async fn links(
        &self,
        ctx: &Context<'_>,
    ) -> Result<impl Stream<Item = Vec<LinkStatusGql>>> {
        let service = svc(ctx)?.clone();
        let mut rx = service.subscribe_links();
        Ok(async_stream::stream! {
            if let Some(s) = service.session() {
                yield s
                    .link_status
                    .into_iter()
                    .map(|l| LinkStatusGql {
                        from: l.from,
                        to: l.to,
                        state: l.state.into(),
                    })
                    .collect();
            }
            loop {
                match rx.recv().await {
                    Ok(links) => {
                        yield links
                            .into_iter()
                            .map(|l| LinkStatusGql {
                                from: l.from,
                                to: l.to,
                                state: l.state.into(),
                            })
                            .collect();
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        })
    }
}
