use abi_stable::std_types::{RString, RVec};
use anyrun_plugin::*;

#[init]
fn init(_: RString) -> () {
    panic!("mock ic in init");
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "mock-panic-init".into(),
        icon: "".into(),
    }
}

#[get_matches]
fn get_matches(_: RString) -> RVec<Match> {
    vec![].into()
}

#[handler]
fn handler(_: Match) -> HandleResult {
    HandleResult::Close
}
