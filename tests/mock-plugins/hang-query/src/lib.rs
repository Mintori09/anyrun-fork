use std::time::Duration;

use abi_stable::std_types::{RString, RVec};
use anyrun_plugin::*;

struct Config;

impl Default for Config {
    fn default() -> Self {
        Config
    }
}

#[init]
fn init(_: RString) -> Config {
    Config
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "mock-hang-query".into(),
        icon: "".into(),
    }
}

#[get_matches]
fn get_matches(_: RString, _: &Config) -> RVec<Match> {
    std::thread::sleep(Duration::from_secs(u64::MAX));
    vec![].into()
}

#[handler]
fn handler(_: Match, _: &Config) -> HandleResult {
    HandleResult::Close
}
