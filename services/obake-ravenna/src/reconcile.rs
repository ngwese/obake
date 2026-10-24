// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Diff desired qualified streams against loaded PipeWire modules.

use crate::pipewire_nodes::{DriverArgs, ModuleArgs, PipewireClient};
use crate::qualify::StreamState;
use std::collections::{HashMap, HashSet};

/// Apply create/update/delete so loaded modules match `want`.
pub fn reconcile(
    client: &mut dyn PipewireClient,
    iface: &str,
    want: &[StreamState],
    current: &mut HashMap<String, ModuleArgs>,
) -> Result<(), String> {
    let desired: HashMap<String, ModuleArgs> = want
        .iter()
        .filter_map(|s| match s {
            StreamState::SourceUp(src) => {
                Some((src.name.clone(), ModuleArgs::from_source(src, iface)))
            }
            StreamState::SinkUp(sink) => {
                Some((sink.name.clone(), ModuleArgs::from_sink(sink, iface)))
            }
            _ => None,
        })
        .collect();

    let desired_names: HashSet<&str> = desired.keys().map(|s| s.as_str()).collect();
    let to_drop: Vec<String> = current
        .keys()
        .filter(|k| !desired_names.contains(k.as_str()))
        .cloned()
        .collect();
    for name in to_drop {
        client.unload_module(&name)?;
        current.remove(&name);
    }

    for (name, args) in &desired {
        match current.get(name) {
            Some(existing) if existing == args => {}
            _ => {
                client.load_module(args)?;
                current.insert(name.clone(), args.clone());
            }
        }
    }
    Ok(())
}

/// Ensure the PTP driver matches `args`, or remove it when `args` is None.
pub fn reconcile_driver(
    client: &mut dyn PipewireClient,
    args: Option<&DriverArgs>,
) -> Result<(), String> {
    match args {
        Some(a) => client.ensure_driver(a),
        None => client.remove_driver(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Format;
    use crate::destination::MulticastDestination;
    use crate::pipewire_nodes::{PwEvent, RecordingPipewire};
    use crate::qualify::QualifiedSource;

    fn source_up(name: &str, port: u16) -> StreamState {
        StreamState::SourceUp(QualifiedSource {
            name: name.into(),
            channels: 2,
            format: Format::L16,
            frame_size: 48,
            destination: MulticastDestination::new(
                std::net::Ipv4Addr::new(239, 1, 60, 50),
                port,
            )
            .unwrap(),
            ttl: 15,
            rate: 48_000,
            domain: 0,
            gmid: [1, 2, 3, 4, 5, 6, 7, 8],
            payload_type: 98,
            node_group: "obake.ravenna".into(),
        })
    }

    #[test]
    fn loads_then_unloads() {
        let mut pw = RecordingPipewire::new(48_000);
        let mut current = HashMap::new();
        reconcile(
            &mut pw,
            "enp3s0",
            &[source_up("a", 5004)],
            &mut current,
        )
        .unwrap();
        assert_eq!(current.len(), 1);
        reconcile(&mut pw, "enp3s0", &[], &mut current).unwrap();
        assert!(current.is_empty());
        assert!(pw
            .events
            .iter()
            .any(|e| matches!(e, PwEvent::Unload(_))));
    }

    #[test]
    fn driver_ensure_once() {
        let mut pw = RecordingPipewire::new(48_000);
        let args = DriverArgs {
            name: "obake-ptp-driver".into(),
            node_group: "obake.ravenna".into(),
            clock_interface: "enp3s0".into(),
        };
        reconcile_driver(&mut pw, Some(&args)).unwrap();
        reconcile_driver(&mut pw, Some(&args)).unwrap();
        assert_eq!(pw.driver.as_ref(), Some(&args));
        reconcile_driver(&mut pw, None).unwrap();
        assert!(pw.driver.is_none());
    }
}
