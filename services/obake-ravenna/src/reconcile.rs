// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Diff qualify results against loaded RTP modules and apply load/unload.

use crate::pipewire_nodes::{ModuleArgs, PipewireClient};
use crate::qualify::{QualifiedSink, QualifiedSource, StreamState};
use std::collections::HashMap;

/// A stream that should have an active PipeWire RTP module.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Desired {
    Source(QualifiedSource),
    Sink(QualifiedSink),
}

/// Apply qualify results to a PipeWire client. Unloads names that disappear or
/// change, then loads the desired set. Does not restart PipeWire.
pub fn reconcile(
    client: &mut dyn PipewireClient,
    iface: &str,
    states: &[StreamState],
    current: &mut HashMap<String, ModuleArgs>,
) -> Result<(), String> {
    let mut desired: HashMap<String, Desired> = HashMap::new();
    for state in states {
        match state {
            StreamState::SourceUp(s) => {
                desired.insert(s.name.clone(), Desired::Source(s.clone()));
            }
            StreamState::SinkUp(s) => {
                desired.insert(s.name.clone(), Desired::Sink(s.clone()));
            }
            _ => {}
        }
    }

    let to_unload: Vec<String> = current
        .keys()
        .filter(|name| {
            let Some(want) = desired.get(*name) else {
                return true;
            };
            let want_args = desired_args(want, iface);
            current.get(*name) != Some(&want_args)
        })
        .cloned()
        .collect();

    for name in &to_unload {
        client.unload_module(name)?;
        current.remove(name);
    }

    for (name, want) in &desired {
        if current.contains_key(name) {
            continue;
        }
        let args = desired_args(want, iface);
        client.load_module(&args)?;
        current.insert(name.clone(), args);
    }
    Ok(())
}

/// Map a desired stream to concrete module load arguments.
fn desired_args(want: &Desired, iface: &str) -> ModuleArgs {
    match want {
        Desired::Source(s) => ModuleArgs::from_source(s, iface),
        Desired::Sink(s) => ModuleArgs::from_sink(s, iface),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Format;
    use crate::destination::MulticastDestination;
    use crate::pipewire_nodes::{PwEvent, RecordingPipewire};
    use crate::qualify::QualifiedSource;
    use crate::sdp::StreamSdp;
    use std::net::Ipv4Addr;

    fn source(name: &str, group: &str) -> StreamState {
        StreamState::SourceUp(QualifiedSource {
            name: name.into(),
            channels: 2,
            format: Format::L24,
            frame_size: 48,
            destination: MulticastDestination::parse(&format!("{group}:5004")).unwrap(),
            ttl: 15,
            rate: 48_000,
            domain: 0,
        })
    }

    fn sink(name: &str, group: Ipv4Addr) -> StreamState {
        StreamState::SinkUp(QualifiedSink {
            name: name.into(),
            device: "Studio-IO".into(),
            stream: "PGM-1".into(),
            sdp: StreamSdp {
                group,
                port: 5004,
                rate: 48_000,
                channels: 2,
                format: Format::L24,
                frame_size: 48,
                domain: 0,
            },
            rate: 48_000,
        })
    }

    #[test]
    fn load_and_unload_by_name() {
        let mut pw = RecordingPipewire::new(48_000);
        let mut current = HashMap::new();
        reconcile(
            &mut pw,
            "enp1s0",
            &[source("out", "239.69.1.10")],
            &mut current,
        )
        .unwrap();
        assert_eq!(pw.events, vec![PwEvent::Load("out".into())]);

        reconcile(&mut pw, "enp1s0", &[], &mut current).unwrap();
        assert_eq!(
            pw.events,
            vec![PwEvent::Load("out".into()), PwEvent::Unload("out".into())]
        );
    }

    #[test]
    fn sink_sdp_group_change_unloads_then_loads() {
        let mut pw = RecordingPipewire::new(48_000);
        let mut current = HashMap::new();
        reconcile(
            &mut pw,
            "enp1s0",
            &[sink("in", Ipv4Addr::new(239, 69, 1, 10))],
            &mut current,
        )
        .unwrap();
        pw.events.clear();
        reconcile(
            &mut pw,
            "enp1s0",
            &[sink("in", Ipv4Addr::new(239, 69, 1, 20))],
            &mut current,
        )
        .unwrap();
        assert_eq!(
            pw.events,
            vec![PwEvent::Unload("in".into()), PwEvent::Load("in".into())]
        );
        assert_eq!(
            current.get("in").unwrap().destination,
            "239.69.1.20"
        );
    }

    #[test]
    fn graph_rate_comes_from_client() {
        let pw = RecordingPipewire::new(48_000);
        assert_eq!(pw.graph_rate(), 48_000);
    }
}
