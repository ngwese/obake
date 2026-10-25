// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Production PipeWire client for the `obake` user daemon.
//!
//! Uses the [`pipewire`] crate (`libpipewire`) on a dedicated mainloop thread.
//! RTP modules are loaded into this process's context (they export streams to
//! the daemon). The mainloop must keep running for those streams to stay
//! connected — commands arrive via [`pipewire::channel`], not a blocking
//! `mpsc::recv` that starves the loop.
//!
//! The PTP driver is created as a remote `spa-node-factory` object so it runs
//! in the daemon and can own the graph clock.
//!
//! Unit tests use [`crate::pipewire_nodes::RecordingPipewire`] and never start
//! this client.

use crate::pipewire_nodes::{DriverArgs, ModuleArgs, ModuleKind, PipewireClient};
use log::{debug, info};
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Build SPA-JSON args for `libpipewire-module-rtp-sink` / `rtp-source`.
pub fn rtp_module_args(args: &ModuleArgs) -> String {
    let (ip_key, port_key) = match args.kind {
        ModuleKind::RtpSink => ("destination.ip", "destination.port"),
        ModuleKind::RtpSource => ("source.ip", "source.port"),
    };
    let media_class = match args.kind {
        ModuleKind::RtpSink => "Audio/Sink",
        ModuleKind::RtpSource => "Audio/Source",
    };
    let ttl_line = args
        .ttl
        .map(|t| format!("            net.ttl          = {t}\n"))
        .unwrap_or_default();
    // Transmit timestamps follow the graph clock, then snap down onto the
    // packet grid so `timestamp % framecount == 0` (AES67 mediaclk:direct=0).
    let quantize = match args.kind {
        ModuleKind::RtpSink => "            sess.ts-quantize   = true\n",
        ModuleKind::RtpSource => "",
    };
    format!(
        "{{\n\
            local.ifname     = \"{iface}\"\n\
            {ip_key}   = \"{dest}\"\n\
            {port_key} = {port}\n\
            net.mtu          = 1500\n\
{ttl}\
            net.loop         = false\n\
            sess.min-ptime     = {ptime}\n\
            sess.max-ptime     = {ptime}\n\
            sess.latency.msec  = {latency}\n\
            sess.name          = \"{name}\"\n\
            sess.media         = \"audio\"\n\
            sess.ts-refclk     = \"{ts}\"\n\
            sess.ts-offset     = {ts_offset}\n\
            sess.ts-direct     = true\n\
{quantize}\
            audio.format   = \"{format}\"\n\
            audio.rate     = {rate}\n\
            audio.channels = {channels}\n\
            stream.props = {{\n\
                node.name           = \"{name}\"\n\
                media.class         = \"{media_class}\"\n\
                node.virtual        = true\n\
                device.api          = \"aes67\"\n\
                node.always-process = true\n\
                node.group          = \"{group}\"\n\
                node.autoconnect    = false\n\
                rtp.framecount      = {framecount}\n\
                rtp.payload         = {payload}\n\
                rtp.ntp             = 0\n\
                rtp.fetch-ts-refclk = true\n\
            }}\n\
        }}",
        iface = args.iface,
        dest = args.destination,
        port = args.port,
        ttl = ttl_line,
        ptime = args.ptime_ms,
        latency = args.sess_latency_msec,
        name = args.name,
        ts = args.ts_refclk,
        ts_offset = args.ts_offset,
        quantize = quantize,
        format = args.audio_format,
        rate = args.rate,
        channels = args.channels,
        media_class = media_class,
        group = args.node_group,
        framecount = args.framecount,
        payload = args.payload_type,
        ip_key = ip_key,
        port_key = port_key,
    )
}

/// PipeWire module library name for a stream kind.
pub fn rtp_module_name(kind: &ModuleKind) -> &'static str {
    match kind {
        ModuleKind::RtpSink => "libpipewire-module-rtp-sink",
        ModuleKind::RtpSource => "libpipewire-module-rtp-source",
    }
}

/// SPA-JSON / property dump for the PTP `support.node.driver` (tests / debug).
pub fn driver_object_args(args: &DriverArgs) -> String {
    format!(
        "{{\n\
            factory.name    = support.node.driver\n\
            node.name       = \"{name}\"\n\
            node.group      = \"{group}\"\n\
            priority.driver = 100000\n\
            clock.interface = \"{iface}\"\n\
            resync.ms       = 5.0\n\
        }}",
        name = args.name,
        group = args.node_group,
        iface = args.clock_interface,
    )
}

enum Cmd {
    EnsureDriver(DriverArgs, mpsc::Sender<Result<(), String>>),
    RemoveDriver(mpsc::Sender<Result<(), String>>),
    Load(ModuleArgs, mpsc::Sender<Result<(), String>>),
    Unload(String, mpsc::Sender<Result<(), String>>),
    Shutdown,
}

/// Live client: libpipewire mainloop thread + command channel.
pub struct SystemPipewire {
    rate: u32,
    tx: pipewire::channel::Sender<Cmd>,
    join: Option<thread::JoinHandle<()>>,
}

impl SystemPipewire {
    /// Connect to the `obake` user PipeWire socket (`PIPEWIRE_RUNTIME_DIR`).
    pub fn connect(rate: u32) -> Result<Self, String> {
        let (tx, rx) = pipewire::channel::channel::<Cmd>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let join = thread::Builder::new()
            .name("obake-ravenna-pw".into())
            .spawn(move || pw_thread(rx, ready_tx))
            .map_err(|e| format!("spawn pipewire thread: {e}"))?;
        ready_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "pipewire thread: timeout waiting for ready".to_string())??;
        info!("pipewire: system client connected (media rate={rate})");
        Ok(Self {
            rate,
            tx,
            join: Some(join),
        })
    }

    fn call<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce(mpsc::Sender<Result<(), String>>) -> Cmd,
    {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(f(reply_tx))
            .map_err(|_| "pipewire thread stopped".to_string())?;
        reply_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| "pipewire thread: command timeout".to_string())?
    }
}

impl Drop for SystemPipewire {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl PipewireClient for SystemPipewire {
    fn ensure_driver(&mut self, args: &DriverArgs) -> Result<(), String> {
        let args = args.clone();
        self.call(|reply| Cmd::EnsureDriver(args, reply))
    }

    fn remove_driver(&mut self) -> Result<(), String> {
        self.call(Cmd::RemoveDriver)
    }

    fn load_module(&mut self, args: &ModuleArgs) -> Result<(), String> {
        let args = args.clone();
        self.call(|reply| Cmd::Load(args, reply))
    }

    fn unload_module(&mut self, name: &str) -> Result<(), String> {
        let name = name.to_string();
        self.call(|reply| Cmd::Unload(name, reply))
    }

    fn graph_rate(&self) -> u32 {
        self.rate
    }
}

fn pw_thread(rx: pipewire::channel::Receiver<Cmd>, ready: mpsc::Sender<Result<(), String>>) {
    use pipewire as pw;
    use pipewire::loop_::Timeout;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    pw::init();

    // All PipeWire objects must be dropped before `pw_deinit`, or Ctrl-C
    // teardown SEGVs after deleting streams/driver.
    {
        let mainloop = match pw::main_loop::MainLoopRc::new(None) {
            Ok(m) => m,
            Err(e) => {
                let _ = ready.send(Err(format!("pipewire mainloop: {e}")));
                return;
            }
        };
        // Context default application.name labels the RTP module's own
        // pw_context_connect (it only passes remote.name). The session
        // connect below overrides with [session].
        let context_props = pw::properties::properties! {
            *pw::keys::APP_NAME => "obake-ravenna [rtp]",
        };
        let context = match pw::context::ContextRc::new(&mainloop, Some(context_props)) {
            Ok(c) => c,
            Err(e) => {
                let _ = ready.send(Err(format!("pipewire context: {e}")));
                return;
            }
        };
        let session_props = pw::properties::properties! {
            *pw::keys::APP_NAME => "obake-ravenna [session]",
        };
        let core = match context.connect_rc(Some(session_props)) {
            Ok(c) => c,
            Err(e) => {
                let _ = ready.send(Err(format!("pipewire connect: {e}")));
                return;
            }
        };

        if let Err(e) = roundtrip(&mainloop, &core) {
            let _ = ready.send(Err(format!("pipewire sync: {e}")));
            return;
        }

        let driver: Rc<RefCell<Option<pw::node::Node>>> = Rc::new(RefCell::new(None));
        let driver_args: Rc<RefCell<Option<DriverArgs>>> = Rc::new(RefCell::new(None));
        let modules: Rc<RefCell<HashMap<String, *mut c_void>>> =
            Rc::new(RefCell::new(HashMap::new()));

        let mainloop_quit = mainloop.clone();
        let core_cmd = core.clone();
        let context_cmd = context.clone();
        let driver_cmd = driver.clone();
        let driver_args_cmd = driver_args.clone();
        let modules_cmd = modules.clone();

        // Keep this attached for the lifetime of mainloop.run().
        let _attached = rx.attach(mainloop.loop_(), move |cmd| {
            match cmd {
                // Only quit here. Destroy leftovers after run() returns so we
                // are not nested inside this IO callback, then drop objects
                // before pw_deinit.
                Cmd::Shutdown => {
                    mainloop_quit.quit();
                }
                Cmd::EnsureDriver(args, reply) => {
                    let r = ensure_driver(
                        &mainloop_quit,
                        &core_cmd,
                        &mut driver_cmd.borrow_mut(),
                        &mut driver_args_cmd.borrow_mut(),
                        &args,
                    );
                    let _ = reply.send(r);
                }
                Cmd::RemoveDriver(reply) => {
                    let r = remove_driver(
                        &mainloop_quit,
                        &core_cmd,
                        &mut driver_cmd.borrow_mut(),
                        &mut driver_args_cmd.borrow_mut(),
                    );
                    let _ = reply.send(r);
                }
                Cmd::Load(args, reply) => {
                    let r = load_module(
                        &mainloop_quit,
                        &core_cmd,
                        &context_cmd,
                        &mut modules_cmd.borrow_mut(),
                        &args,
                    );
                    let _ = reply.send(r);
                }
                Cmd::Unload(name, reply) => {
                    let r = unload_module(
                        &mainloop_quit,
                        &core_cmd,
                        &mut modules_cmd.borrow_mut(),
                        &name,
                    );
                    let _ = reply.send(r);
                }
            }
        });

        let _ = ready.send(Ok(()));
        mainloop.run();

        // run() returned (Shutdown). Clear any objects still held, then let
        // the block end so proxies/context/loop drop before deinit.
        let names: Vec<String> = modules.borrow().keys().cloned().collect();
        for name in names {
            let _ = unload_module(&mainloop, &core, &mut modules.borrow_mut(), &name);
        }
        let _ = remove_driver(
            &mainloop,
            &core,
            &mut driver.borrow_mut(),
            &mut driver_args.borrow_mut(),
        );
    }

    unsafe {
        pw::deinit();
    }

    /// Drain the loop until a core sync completes.
    ///
    /// Safe before `mainloop.run()` and from an attached command callback
    /// (nested `iterate` while already inside `run`).
    fn roundtrip(
        mainloop: &pw::main_loop::MainLoopRc,
        core: &pw::core::CoreRc,
    ) -> Result<(), String> {
        let done = Rc::new(Cell::new(false));
        let done_clone = done.clone();
        let pending = core.sync(0).map_err(|e| format!("sync: {e}"))?;
        let _listener = core
            .add_listener_local()
            .done(move |id, seq| {
                if id == pw::core::PW_ID_CORE && seq == pending {
                    done_clone.set(true);
                }
            })
            .register();
        while !done.get() {
            mainloop.loop_().iterate(Timeout::Infinite);
        }
        Ok(())
    }

    fn ensure_driver(
        mainloop: &pw::main_loop::MainLoopRc,
        core: &pw::core::CoreRc,
        driver: &mut Option<pw::node::Node>,
        driver_args: &mut Option<DriverArgs>,
        args: &DriverArgs,
    ) -> Result<(), String> {
        if driver_args.as_ref() == Some(args) && driver.is_some() {
            return Ok(());
        }
        let action = if driver.is_some() { "update" } else { "create" };
        info!("pipewire: {action} driver name={}", args.name);
        debug!(
            "pipewire: driver props name={} group={} clock.interface={}",
            args.name, args.node_group, args.clock_interface
        );
        if driver.is_some() {
            remove_driver(mainloop, core, driver, driver_args)?;
        }
        let props = pw::properties::properties! {
            "factory.name" => "support.node.driver",
            "node.name" => &*args.name,
            "node.group" => &*args.node_group,
            "priority.driver" => "100000",
            "clock.interface" => &*args.clock_interface,
            // 1.5ms (=72 samples @48k) was just under observed PHC error
            // (~74 samples), causing continuous spa.driver resets and SAC
            // jumps that mute AES67 receivers.
            "resync.ms" => "5.0",
        };
        let node = core
            .create_object::<pw::node::Node>("spa-node-factory", &props)
            .map_err(|e| format!("create driver: {e}"))?;
        roundtrip(mainloop, core)?;
        *driver = Some(node);
        *driver_args = Some(args.clone());
        Ok(())
    }

    fn remove_driver(
        mainloop: &pw::main_loop::MainLoopRc,
        core: &pw::core::CoreRc,
        driver: &mut Option<pw::node::Node>,
        driver_args: &mut Option<DriverArgs>,
    ) -> Result<(), String> {
        if let Some(node) = driver.take() {
            let name = driver_args
                .as_ref()
                .map(|a| a.name.as_str())
                .unwrap_or("?");
            info!("pipewire: delete driver name={name}");
            let _ = core.destroy_object(node);
            let _ = roundtrip(mainloop, core);
        }
        *driver_args = None;
        Ok(())
    }

    fn load_module(
        mainloop: &pw::main_loop::MainLoopRc,
        core: &pw::core::CoreRc,
        context: &pw::context::ContextRc,
        modules: &mut HashMap<String, *mut c_void>,
        args: &ModuleArgs,
    ) -> Result<(), String> {
        let action = if modules.contains_key(&args.name) {
            "update"
        } else {
            "create"
        };
        info!("pipewire: {action} stream name={}", args.name);
        debug!("pipewire: stream props {}", args.debug_props());
        if modules.contains_key(&args.name) {
            unload_module(mainloop, core, modules, &args.name)?;
        }
        let spa = rtp_module_args(args);
        let mod_name = rtp_module_name(&args.kind);
        let c_name = CString::new(mod_name).map_err(|e| e.to_string())?;
        let c_args = CString::new(spa).map_err(|e| e.to_string())?;
        let module = unsafe {
            pw::sys::pw_context_load_module(
                context.as_raw_ptr(),
                c_name.as_ptr(),
                c_args.as_ptr(),
                std::ptr::null_mut(),
            )
        };
        if module.is_null() {
            return Err(format!("load {mod_name} failed"));
        }
        let _ = roundtrip(mainloop, core);
        modules.insert(args.name.clone(), module as *mut c_void);
        Ok(())
    }

    fn unload_module(
        mainloop: &pw::main_loop::MainLoopRc,
        core: &pw::core::CoreRc,
        modules: &mut HashMap<String, *mut c_void>,
        name: &str,
    ) -> Result<(), String> {
        if let Some(ptr) = modules.remove(name) {
            info!("pipewire: delete stream name={name}");
            unsafe {
                pw::sys::pw_impl_module_destroy(ptr as *mut pw::sys::pw_impl_module);
            }
            let _ = roundtrip(mainloop, core);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Format;
    use crate::destination::MulticastDestination;
    use crate::qualify::QualifiedSource;

    #[test]
    fn spa_args_put_rtp_keys_in_stream_props() {
        let src = QualifiedSource {
            name: "obake-out-1".into(),
            channels: 8,
            format: Format::L24,
            frame_size: 48,
            destination: MulticastDestination::parse("239.1.60.50:5004").unwrap(),
            ttl: 15,
            rate: 48_000,
            domain: 0,
            gmid: [1, 2, 3, 4, 5, 6, 7, 8],
            payload_type: 104,
            node_group: "obake.ravenna".into(),
            source_ip: std::net::Ipv4Addr::new(192, 168, 60, 50),
        };
        let args = ModuleArgs::from_source(&src, "enp3s0");
        let spa = rtp_module_args(&args);
        assert!(spa.contains("rtp.framecount      = 48"));
        assert!(spa.contains("rtp.payload         = 104"));
        assert!(spa.contains("node.group          = \"obake.ravenna\""));
        assert!(spa.contains("sess.ts-direct     = true"));
        assert!(spa.contains("sess.ts-quantize   = true"));
        assert!(!spa.contains("aes67.driver-group"));
        assert!(!spa.contains("object.export"));
        assert!(!spa.contains("clock.force-quantum"));
        assert_eq!(
            args.ts_refclk,
            "ptp=IEEE1588-2008:01-02-03-04-05-06-07-08:0"
        );
        assert_eq!(rtp_module_name(&args.kind), "libpipewire-module-rtp-sink");
    }

    #[test]
    fn driver_args_omit_export_linger_and_quantum() {
        let args = DriverArgs {
            name: "obake-ptp-driver".into(),
            node_group: "obake.ravenna".into(),
            clock_interface: "enp3s0".into(),
        };
        let spa = driver_object_args(&args);
        assert!(spa.contains("support.node.driver"));
        assert!(spa.contains("clock.interface = \"enp3s0\""));
        assert!(!spa.contains("object.export"));
        assert!(!spa.contains("object.linger"));
        assert!(!spa.contains("clock.force-quantum"));
    }
}
