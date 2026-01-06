use std::{
    cell::RefCell,
    io::{self, IsTerminal, Read, Write},
    rc::Rc,
};

use clap::{Parser, Subcommand};
use gtk4::{self as gtk, gio, glib, prelude::*};
use relm4::Sender;
use serde::{Deserialize, Serialize};

mod app;
mod config;
mod plugin_box;
mod provider;

use crate::config::ConfigArgs;

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
        _obj_path: &str,
        _interface: Option<&str>,
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
                "No such method",
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
                let method = if matches!(cmd, Command::Close) {
                    "Close"
                } else {
                    "Quit"
                };
                fast_ipc_call(method);
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

fn fast_ipc_call(method: &str) {
    let conn = gio::bus_get_sync(gio::BusType::Session, Option::<&gio::Cancellable>::None)
        .expect("Failed to connect to DBus session bus");

    let _ = conn.call_sync(
        Some("org.anyrun.anyrun"),
        "/org/anyrun/anyrun",
        "org.anyrun.Anyrun",
        method,
        None,
        None,
        gio::DBusCallFlags::NONE,
        500,
        Option::<&gio::Cancellable>::None,
    );
}

fn run_client(args: Args) {
    let app = gtk::Application::new(Some("org.anyrun.anyrun"), Default::default());
    app.register(Option::<&gio::Cancellable>::None).unwrap();

    if app.is_remote() {
        let mut stdin = Vec::new();
        if !io::stdin().is_terminal() {
            let _ = io::stdin().read_to_end(&mut stdin);
        }

        let env: Vec<(String, String)> = std::env::vars().collect();

        let conn = app.dbus_connection().unwrap();
        let init_payload = app::AppInit { args, stdin, env };
        let serialized_data = serde_json::to_vec(&init_payload).unwrap();

        let msg = (serialized_data,).to_variant();

        let res = conn
            .call_sync(
                Some("org.anyrun.anyrun"),
                "/org/anyrun/anyrun",
                "org.anyrun.Anyrun",
                "Show",
                Some(&msg),
                None,
                gio::DBusCallFlags::NONE,
                -1,
                Option::<&gio::Cancellable>::None,
            )
            .expect("Daemon call failed");

        let Some(bytes) = res.child_value(0).get::<Vec<u8>>() else {
            return;
        };

        let Ok(app::PostRunAction::Stdout(stdout)) =
            serde_json::from_slice::<app::PostRunAction>(&bytes)
        else {
            return;
        };

        let _ = io::stdout().lock().write_all(&stdout);
    } else {
        let mut stdin = Vec::new();
        if !io::stdin().is_terminal() {
            let _ = io::stdin().read_to_end(&mut stdin);
        }
        let env: Vec<(String, String)> = std::env::vars().collect();

        app.connect_activate(move |app| {
            app::App::launch(
                app,
                app::AppInit {
                    args: args.clone(),
                    stdin: stdin.clone(),
                    env: env.clone(),
                },
                None,
            );
        });
        app.run_with_args(&Vec::<String>::new());
    }
}

fn run_daemon(_args: Args) {
    let app = gtk::Application::new(Some("org.anyrun.anyrun"), gio::ApplicationFlags::IS_SERVICE);
    app.register(Option::<&gio::Cancellable>::None).unwrap();

    let _hold = app.hold();
    let state = Rc::new(RefCell::new(DaemonState { sender: None }));
    let dbus_conn = app.dbus_connection().unwrap();

    let node_info = gio::DBusNodeInfo::for_xml(INTERFACE_XML).unwrap();
    let interface = node_info.lookup_interface("org.anyrun.Anyrun").unwrap();

    dbus_conn
        .register_object("/org/anyrun/anyrun", &interface)
        .typed_method_call::<InterfaceMethod>()
        .invoke(glib::clone!(
            #[weak]
            app,
            #[strong]
            state,
            move |_conn, _sender, method, invocation| {
                match method {
                    InterfaceMethod::Show(show) => {
                        let init_data = serde_json::from_slice(&show.args).unwrap();
                        state.borrow_mut().sender =
                            Some(app::App::launch(&app, init_data, Some(invocation)));
                    }
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
        .unwrap();

    app.run_with_args(&Vec::<String>::new());
}
