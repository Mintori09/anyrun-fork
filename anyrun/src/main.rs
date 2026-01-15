use clap::{Parser, Subcommand};
use gtk4::{self as gtk, gio, glib, prelude::*};
use relm4::Sender;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    io::{self, IsTerminal, Read, Write},
    rc::Rc,
    sync::Arc,
};

mod app;
mod config;
mod plugin_box;
mod provider;
use crate::config::ConfigArgs;
use gio::prelude::DBusMethodCall;

const INTERFACE_XML: &str = r#"
<node>
    <interface name="org.anyrun.Anyrun">
        <method name="Show">
            <arg type="ay" name="args" direction="in"/>
            <arg type="ay" name="result" direction="out"/>
        </method>
        <method name="Close"></method>
        <method name="Quit"></method>
    </interface>
</node>
"#;

#[derive(Debug, glib::Variant)]
struct Show {
    args: Vec<u8>,
}

enum InterfaceMethod {
    Show(Show),
    Close,
    Quit,
}

impl DBusMethodCall for InterfaceMethod {
    fn parse_call(
        _obj: &str,
        _intf: Option<&str>,
        method: &str,
        params: glib::Variant,
    ) -> Result<Self, glib::Error> {
        match method {
            "Show" => params
                .get::<Show>()
                .map(Self::Show)
                .ok_or_else(|| glib::Error::new(gio::DBusError::InvalidArgs, "Invalid args")),
            "Close" => Ok(Self::Close),
            "Quit" => Ok(Self::Quit),
            _ => Err(glib::Error::new(
                gio::DBusError::UnknownMethod,
                "Unknown method",
            )),
        }
    }
}

#[derive(Parser, Clone, Debug, Serialize, Deserialize)]
#[command(version, about)]
pub struct Args {
    #[arg(short, long)]
    config_dir: Option<String>,
    #[command(flatten)]
    config: ConfigArgs,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone, Copy, Deserialize, Serialize)]
enum Command {
    Daemon,
    Close,
    Quit,
}

struct DaemonState {
    sender: Option<Sender<app::AppMsg>>,
}

fn main() {
    let args = Args::parse();

    if let Some(cmd) = args.command {
        match cmd {
            Command::Close | Command::Quit => {
                fast_ipc_call(if matches!(cmd, Command::Close) {
                    "Close"
                } else {
                    "Quit"
                });
                return;
            }
            Command::Daemon => {
                run_daemon(args);
                return;
            }
        }
    }
    run_client(args);
}

fn fast_ipc_call(method: &'static str) {
    gio::bus_get(
        gio::BusType::Session,
        None::<&gio::Cancellable>,
        move |res| {
            if let Ok(conn) = res {
                conn.call(
                    Some("org.anyrun.anyrun"),
                    "/org/anyrun/anyrun",
                    "org.anyrun.Anyrun",
                    method,
                    None,
                    None,
                    gio::DBusCallFlags::NO_AUTO_START,
                    1_000,
                    None::<&gio::Cancellable>,
                    |_| {},
                );
            }
        },
    );
}

fn run_client(args: Args) {
    let app = gtk::Application::new(Some("org.anyrun.anyrun"), gio::ApplicationFlags::FLAGS_NONE);
    if let Err(e) = app.register(None::<&gio::Cancellable>) {
        eprintln!("Registration error: {e}");
        return;
    }

    let read_init_data = || {
        let mut stdin = Vec::new();
        if !io::stdin().is_terminal() {
            io::stdin()
                .lock()
                .take(2 * 1024 * 1024)
                .read_to_end(&mut stdin)
                .ok();
        }
        let env: Vec<(String, String)> = std::env::vars().collect();
        app::AppInit { args, stdin, env }
    };

    if app.is_remote() {
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
        main_loop.run();
    } else {
        let shared_init = Arc::new(read_init_data());

        app.connect_activate(move |app| {
            app::App::launch(app, (*shared_init).clone(), None);
        });
        app.run_with_args(&Vec::<String>::new());
    }
}

fn run_daemon(_args: Args) {
    let app = gtk::Application::new(Some("org.anyrun.anyrun"), gio::ApplicationFlags::IS_SERVICE);
    app.register(None::<&gio::Cancellable>)
        .expect("Failed to register daemon");

    let _hold = app.hold();
    let state = Rc::new(RefCell::new(DaemonState { sender: None }));
    let dbus_conn = app
        .dbus_connection()
        .expect("Failed to get DBus connection");

    let node_info = gio::DBusNodeInfo::for_xml(INTERFACE_XML).expect("Invalid XML");
    let interface = node_info.lookup_interface("org.anyrun.Anyrun").unwrap();

    dbus_conn
        .register_object("/org/anyrun/anyrun", &interface)
        .typed_method_call::<InterfaceMethod>()
        .invoke(glib::clone!(
            #[weak]
            app,
            #[strong]
            state,
            move |_, _, method, invocation| {
                match method {
                    InterfaceMethod::Show(show) => match serde_json::from_slice(&show.args) {
                        Ok(init_data) => {
                            state.borrow_mut().sender =
                                Some(app::App::launch(&app, init_data, Some(invocation)));
                        }
                        Err(_) => {
                            invocation.return_error(gio::DBusError::InvalidArgs, "Invalid JSON");
                        }
                    },
                    InterfaceMethod::Close => {
                        if let Some(s) = &state.borrow().sender {
                            s.emit(app::AppMsg::Action(config::Action::Close));
                        }
                        state.borrow_mut().sender = None;
                        invocation.return_value(None);
                    }
                    InterfaceMethod::Quit => {
                        invocation.return_value(None);
                        app.quit();
                    }
                }
            }
        ))
        .build()
        .expect("Failed to register object");

    app.run_with_args(&Vec::<String>::new());
}
