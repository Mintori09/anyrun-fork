use crate::app;
use crate::args::Args;
use crate::dbus::setup_dbus;
use crate::dbus::DaemonState;
use gtk4::{gio, glib, prelude::*};
use relm4::ComponentController;
use std::cell::RefCell;
use std::io::{self, IsTerminal, Read, Write};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

pub fn run_client(args: Args) {
    let time = Instant::now();
    let duration = time.elapsed();
    println!("[Start Application at {:?}]", duration);

    let app = gtk4::Application::new(Some("org.anyrun.anyrun"), gio::ApplicationFlags::FLAGS_NONE);
    if let Err(e) = app.register(None::<&gio::Cancellable>) {
        eprintln!("Registration error: {e}");
        return;
    }

    let duration = time.elapsed();
    println!("[Call dbus at {:?}]", duration);

    let static_load = args.config.has_plugins();

    let read_init_data = || {
        let mut stdin = Vec::new();
        if !io::stdin().is_terminal() {
            io::stdin()
                .lock()
                .take(2 * 1024 * 1024)
                .read_to_end(&mut stdin)
                .ok();
        }
        let env: Vec<(String, String)> = std::env::vars()
            .filter(|(k, _)| {
                k == "HOME"
                    || k.starts_with("XDG_")
                    || k == "PATH"
                    || k == "DISPLAY"
                    || k == "WAYLAND_DISPLAY"
                    || k.starts_with("ANYRUN_")
                    || k == "LANG"
                    || k == "TERM"
            })
            .collect();
        app::AppInit { args, stdin, env }
    };

    let duration = time.elapsed();
    println!("[Read init Data at {:?}]", duration);

    if !static_load && app.is_remote() {
        let conn = app.dbus_connection().expect("No D-Bus connection");
        let payload = read_init_data();

        let serialized = serde_json::to_vec(&payload).unwrap();
        let bytes = glib::Bytes::from_owned(serialized);
        let msg = glib::Variant::from_bytes::<(Vec<u8>,)>(&bytes);

        let main_loop = glib::MainLoop::new(None, false);
        let loop_clone = main_loop.clone();

        conn.call(
            Some("org.anyrun.anyrun"),
            "/org/anyrun/anyrun",
            "org.anyrun.Anyrun",
            "Show",
            Some(&msg),
            None,
            gio::DBusCallFlags::NONE,
            -1,
            None::<&gio::Cancellable>,
            move |res| {
                if let Ok(val) = res {
                    if let Some(b) = val.child_value(0).get::<Vec<u8>>() {
                        if let Ok(app::PostRunAction::Stdout(out_data)) =
                            serde_json::from_slice::<app::PostRunAction>(&b)
                        {
                            let mut out = io::stdout().lock();
                            let _ = out.write_all(&out_data);
                            let _ = out.flush();
                        }
                    }
                }
                loop_clone.quit();
            },
        );
        let duration = time.elapsed();
        println!("[Check at {:?}]", duration);

        main_loop.run();
    } else {
        let shared_init = Arc::new(read_init_data());

        app.connect_activate(move |app| {
            let controller = app::App::launch(app, (*shared_init).clone(), None, None);
            let state = Rc::new(RefCell::new(DaemonState {
                sender: controller.sender().clone(),
                provider_child: None,
            }));
            setup_dbus(app, state);
            controller.sender().emit(app::AppMsg::Activate(None));
        });
        app.run_with_args(&Vec::<String>::new());
    }
}
