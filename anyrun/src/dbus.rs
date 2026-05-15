use crate::app;
use gio::prelude::DBusMethodCall;
use gtk4::{gio, glib, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;

pub const INTERFACE_XML: &str = r#"
<node>
    <interface name="org.anyrun.Anyrun">
        <method name="Show">
            <arg type="ay" name="args" direction="in"/>
            <arg type="ay" name="result" direction="out"/>
        </method>
        <method name="Close"></method>
        <method name="Quit"></method>
        <method name="Reload"></method>
    </interface>
</node>
"#;

#[derive(Debug, glib::Variant)]
pub struct Show {
    pub args: Vec<u8>,
}

pub enum InterfaceMethod {
    Show(Show),
    Close,
    Quit,
    Reload,
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
            "Reload" => Ok(Self::Reload),
            _ => Err(glib::Error::new(
                gio::DBusError::UnknownMethod,
                "Unknown method",
            )),
        }
    }
}

pub struct DaemonState {
    pub sender: relm4::Sender<app::AppMsg>,
    pub provider_child: Option<std::process::Child>,
}

pub fn fast_ipc_call(method: &'static str) {
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

pub fn setup_dbus(app: &gtk4::Application, state: Rc<RefCell<DaemonState>>) {
    let dbus_conn = match app.dbus_connection() {
        Some(conn) => conn,
        None => return,
    };

    let node_info = gio::DBusNodeInfo::for_xml(INTERFACE_XML).expect("Invalid XML");
    let interface = node_info.lookup_interface("org.anyrun.Anyrun").unwrap();

    let _ = dbus_conn
        .register_object("/org/anyrun/anyrun", &interface)
        .typed_method_call::<InterfaceMethod>()
        .invoke(glib::clone!(
            #[weak]
            app,
            #[strong]
            state,
            move |_, _, method, invocation| {
                match method {
                    InterfaceMethod::Show(show) => {
                        match serde_json::from_slice::<app::AppInit>(&show.args) {
                            Ok(_init_data) => {
                                state.borrow().sender.emit(app::AppMsg::Activate(Some(
                                    app::SendInvocation(invocation),
                                )));
                            }
                            Err(_) => {
                                invocation
                                    .return_error(gio::DBusError::InvalidArgs, "Invalid JSON");
                            }
                        }
                    }
                    InterfaceMethod::Close => {
                        state
                            .borrow()
                            .sender
                            .emit(app::AppMsg::Action(crate::config::Action::Close));
                        invocation.return_value(None);
                    }
                    InterfaceMethod::Quit => {
                        invocation.return_value(None);
                        if let Some(mut child) = state.borrow_mut().provider_child.take() {
                            let _ = child.kill();
                        }
                        app.quit();
                    }
                    InterfaceMethod::Reload => {
                        state.borrow().sender.emit(app::AppMsg::ReloadPlugins);
                        invocation.return_value(None);
                    }
                }
            }
        ))
        .build();
}
