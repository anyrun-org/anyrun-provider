use anyrun_interface::{Match, PluginInfo, RVec};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum Request {
    /// Load & Intialize the plugins.
    /// The provider will respond with a `Response::Intialized`
    Init {
        plugins: Vec<String>

            },
    Query(String)
}

#[derive(Serialize, Deserialize)]
enum Response {
    Initialized {
        info: Vec<PluginInfo>
    },
    Matches {
        plugin: PluginInfo,
        matches: RVec<Match>
    }
}
