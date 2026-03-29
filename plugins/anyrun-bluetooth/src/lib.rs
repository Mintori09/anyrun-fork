use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use bluer::{Adapter, Address, Session};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tokio::runtime::Runtime;

const PLUGIN_NAME: &str = "libbluetooth.so";
const ACTION_ENABLE: u64 = 1;
const ACTION_DISABLE: u64 = 2;
const ACTION_DISCOVERY: u64 = 3;
const DEVICE_ID_OFFSET: u64 = 100;

const ICON_BLUETOOTH_ACTIVE: &str = "bluetooth-active-symbolic";
const ICON_BLUETOOTH_DISABLED: &str = "bluetooth-disabled-symbolic";

#[derive(Deserialize)]
struct Config {
    prefix: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: "bt ".into(),
        }
    }
}

pub struct State {
    config: Config,
    runtime: Runtime,
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = PathBuf::from(config_dir.to_string()).join(PLUGIN_NAME);
    let config = load_config(config_path);
    State {
        config,
        runtime: Runtime::new().expect("Failed to create tokio runtime"),
    }
}

fn load_config(path: PathBuf) -> Config {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_default()
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Bluetooth Control".into(),
        icon: ICON_BLUETOOTH_ACTIVE.into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    state.runtime.block_on(async {
        let mut matches = Vec::new();
        if let Ok(session) = Session::new().await
            && let Ok(adapter) = session.default_adapter().await
        {
            populate_adapter_actions(&adapter, &mut matches).await;
        }
        matches.into()
    })
}

async fn populate_adapter_actions(adapter: &Adapter, matches: &mut Vec<Match>) {
    let is_powered = adapter.is_powered().await.unwrap_or(false);

    if !is_powered {
        matches.push(make_match(
            "Enable Bluetooth",
            "Power on the adapter",
            ICON_BLUETOOTH_ACTIVE,
            ACTION_ENABLE,
        ));
        return;
    }

    if let Ok(addresses) = adapter.device_addresses().await {
        for (idx, addr) in addresses.into_iter().enumerate() {
            if let Ok(device) = adapter.device(addr) {
                let name = device
                    .name()
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| addr.to_string());

                let connected = device.is_connected().await.unwrap_or(false);
                let status = if connected { "Connected" } else { "Paired" };
                let battery = device.battery_percentage().await.ok().flatten();
                let battery_info = battery
                    .map(|p| format!(", Battery: {}%", p))
                    .unwrap_or_default();
                let match_id = DEVICE_ID_OFFSET + idx as u64;

                matches.push(make_match(
                    &name,
                    &format!("Status: {} ({}{})", status, addr, battery_info),
                    ICON_BLUETOOTH_ACTIVE,
                    match_id,
                ));
            }
        }
    }

    matches.push(make_match(
        "Find and match new devices",
        "Discover devices",
        ICON_BLUETOOTH_ACTIVE,
        ACTION_DISCOVERY,
    ));

    matches.push(make_match(
        "Disable Bluetooth",
        "Power off the adapter",
        ICON_BLUETOOTH_DISABLED,
        ACTION_DISABLE,
    ));
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let id = selection.id.unwrap_or(0);

    state.runtime.block_on(async {
        if let Ok(session) = Session::new().await
            && let Ok(adapter) = session.default_adapter().await
        {
            match id {
                ACTION_ENABLE => {
                    let _ = adapter.set_powered(true).await;
                }
                ACTION_DISABLE => {
                    let _ = adapter.set_powered(false).await;
                }
                ACTION_DISCOVERY => {
                    let _ = std::process::Command::new("bluedevil-wizard").spawn();
                }
                idx if idx >= DEVICE_ID_OFFSET => {
                    toggle_device_connection(&adapter, selection).await;
                }
                _ => {}
            }
        }
    });

    HandleResult::Close
}

async fn toggle_device_connection(adapter: &Adapter, selection: Match) {
    let description = selection.description.unwrap_or_default().to_string();
    let address_str = description
        .split('(')
        .next_back()
        .and_then(|s| s.strip_suffix(')'))
        .and_then(|s| s.split(',').next());

    if let Some(addr_str) = address_str
        && let Ok(addr) = addr_str.parse::<Address>()
        && let Ok(device) = adapter.device(addr)
    {
        if device.is_connected().await.unwrap_or(false) {
            let _ = device.disconnect().await;
        } else {
            let _ = device.connect().await;
        }
    }
}

fn make_match(title: &str, description: &str, icon: &str, id: u64) -> Match {
    Match {
        title: title.into(),
        description: ROption::RSome(description.into()),
        use_pango: false,
        icon: ROption::RSome(icon.into()),
        id: ROption::RSome(id),
    }
}
