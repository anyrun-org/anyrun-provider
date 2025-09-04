use anyrun_interface::{HandleResult, Match, PluginInfo, abi_stable::std_types::RVec};
use serde::{Deserialize, Serialize};

/// Requests from subscriber to provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Reset the state of plugins.
    /// Useful for long lived provider processes where the plugin composition
    /// does not change.
    Reset,
    /// Query results from the plugins
    Query {
        /// The text to send to the plugins
        text: String,
    },
    /// Handle a selection using a plugin
    Handle {
        plugin: PluginInfo,
        selection: Match,
    },
    /// Close the provider
    Quit,
}

/// Responses from provider to subscriber
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Sent when a subscriber connects
    Ready {
        /// The list of the plugin info as reported by the plugins, in the same order
        /// as the paths provided with `Request::Init`.
        ///
        /// NOTE: In case of load failures, the vec may be shorter than the provided vec
        info: Vec<PluginInfo>,
        // /// List of possible errors during intialization
        // ///
        // /// TODO: Perhaps unnecessary
        // errors: Vec<String>,
    },
    /// A response to a `Request::Query`. One of these will be received for each plugin per query.
    Matches {
        /// The plugin these matches belong to
        plugin: PluginInfo,
        /// The matches
        matches: RVec<Match>,
    },
    /// A response to a `Request::Handle`
    Handled {
        /// The plugin that handled the selection
        plugin: PluginInfo,
        /// The result provided by the plugin
        result: HandleResult,
    },
}

/// Possible errors reported by the provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    /// The provider can only serve one subscriber. This will be returned if another subscriber
    /// is connected
    Occupied,
}
