use abi_stable::std_types::{ROption, RString, RVec};
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
        name: "mock-basic".into(),
        icon: "".into(),
    }
}

#[get_matches]
fn get_matches(_: RString, _: &Config) -> RVec<Match> {
    vec![Match {
        title: "test-match".into(),
        icon: ROption::RNone,
        id: ROption::RSome(42u64),
        description: ROption::RSome("A test match".into()),
        use_pango: false,
    }]
    .into()
}

#[handler]
fn handler(_: Match, _: &Config) -> HandleResult {
    HandleResult::Copy(b"hello-world".to_vec().into())
}
