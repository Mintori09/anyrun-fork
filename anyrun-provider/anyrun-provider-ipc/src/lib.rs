use std::error::Error as StdError;
use std::io;

use anyrun_interface::{HandleResult, Match, PluginInfo, abi_stable::std_types::RVec};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

/// Maximum allowed frame size for incoming messages (64 MiB)
const MAX_FRAME_SIZE: usize = 64 * 1024 * 1024;

// Default search paths, maintain backwards compatibility
pub const CONFIG_DIRS: &[&str] = &["/etc/xdg/anyrun", "/etc/anyrun"];
pub const PLUGIN_PATHS: &[&str] = &["/usr/lib/anyrun", "/etc/anyrun/plugins"];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryPhase {
    Typing,
    Settling,
}

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
        /// Search phase for the query.
        phase: QueryPhase,
        /// Limit execution to these plugin names when non-empty.
        plugins: Vec<String>,
        /// Maximum time to wait for each plugin during this query.
        timeout_ms: u64,
        /// Threshold for reporting a plugin as slow.
        slow_ms: u64,
    },
    /// Return recently selected matches.
    Recent {
        /// Maximum number of recent matches to return.
        limit: usize,
    },
    /// Handle a selection using a plugin
    Handle {
        plugin: PluginInfo,
        selection: Match,
    },
    /// Close the provider
    Quit,
    /// Reload plugin data (e.g. after desktop entries changed).
    /// The provider should re-init all plugins and send back
    /// a new `Ready` response with updated plugin infos.
    ReloadPlugins {
        /// Optional plugin paths to reload from config. Empty means re-init current set.
        plugins: Vec<String>,
    },
    /// Reload a specific plugin by name.
    /// Useful when a plugin's config file changes.
    ReloadPlugin {
        /// The plugin name (e.g. "calc" for "calc.ron")
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentMatch {
    pub plugin: PluginInfo,
    pub selection: Match,
    pub last_used_unix: i64,
    pub uses: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginHealthState {
    Healthy,
    Slow,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    pub plugin: String,
    pub state: PluginHealthState,
    pub elapsed_ms: u64,
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
    /// Recently selected matches.
    Recent { matches: Vec<RecentMatch> },
    /// Plugin health updates from query execution.
    Health { statuses: Vec<PluginHealth> },
}

/// Possible errors reported by the provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    /// The provider can only serve one subscriber. This will be returned if another subscriber
    /// is connected
    Occupied,
}

pub struct Socket {
    pub inner: BufReader<UnixStream>,
    send_buf: Vec<u8>,
}

impl Socket {
    pub fn new(stream: UnixStream) -> Self {
        let inner = BufReader::new(stream);

        Self {
            inner,
            send_buf: Vec::with_capacity(4096),
        }
    }

    pub async fn send<T: Serialize>(&mut self, value: &T) -> io::Result<()> {
        self.send_buf.clear();
        bincode::serialize_into(&mut self.send_buf, value).map_err(io::Error::other)?;
        let len = self.send_buf.len() as u32;
        self.inner.get_mut().write_all(&len.to_le_bytes()).await?;
        self.inner.get_mut().write_all(&self.send_buf).await?;
        self.inner.get_mut().flush().await?;
        Ok(())
    }

    pub async fn recv<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        let mut len_buf = [0u8; 4];
        self.inner.read_exact(&mut len_buf).await?;
        let len = u32::from_le_bytes(len_buf) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame too large",
            ));
        }
        let mut buf = vec![0u8; len];
        self.inner.read_exact(&mut buf).await?;
        bincode::deserialize::<T>(&buf).map_err(|e| match *e {
            bincode::ErrorKind::Io(io_err) => io_err,
            other => io::Error::other(Box::new(other)),
        })
    }
}

/// Check whether an `io::Error` (at any depth in the error chain) indicates
/// a normal IPC peer disconnect (UnexpectedEof or ConnectionReset).
///
/// This walks the full [`std::error::Error::source`] chain to catch wrapped
/// errors, e.g. `io::Error::other()` wrapping a `bincode::Error` that wraps
/// an inner `io::Error` with `UnexpectedEof`.
pub fn is_ipc_disconnect(err: &io::Error) -> bool {
    let mut current: Option<&dyn StdError> = Some(err);
    while let Some(e) = current {
        if let Some(io) = e.downcast_ref::<io::Error>() {
            match io.kind() {
                io::ErrorKind::UnexpectedEof | io::ErrorKind::ConnectionReset => return true,
                _ => {}
            }
        }
        current = e.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reload_plugins_request_serializes() {
        let req = Request::ReloadPlugins {
            plugins: vec!["libapplications.so".into()],
        };
        let bytes = bincode::serialize(&req).unwrap();
        let deserialized: Request = bincode::deserialize(&bytes).unwrap();
        assert!(matches!(
            deserialized,
            Request::ReloadPlugins { plugins } if plugins == vec!["libapplications.so"]
        ));
    }

    #[test]
    fn test_all_request_variants_roundtrip() {
        let requests = vec![
            Request::Reset,
            Request::Query {
                text: "test".into(),
                phase: QueryPhase::Typing,
                plugins: vec!["Applications".into()],
                timeout_ms: 800,
                slow_ms: 250,
            },
            Request::Recent { limit: 8 },
            Request::Quit,
            Request::ReloadPlugins {
                plugins: vec!["Applications".into()],
            },
        ];
        for req in requests {
            let bytes = bincode::serialize(&req).unwrap();
            let deserialized: Request = bincode::deserialize(&bytes).unwrap();
            assert_eq!(format!("{req:?}"), format!("{deserialized:?}"));
        }
    }

    #[test]
    fn test_recent_and_health_responses_roundtrip() {
        use anyrun_interface::{Match, PluginInfo, abi_stable::std_types::ROption};

        let plugin = PluginInfo {
            name: "Applications".into(),
            icon: "system-search".into(),
        };
        let selection = Match {
            title: "Firefox".into(),
            description: ROption::RNone,
            use_pango: false,
            icon: ROption::RNone,
            id: ROption::RNone,
        };

        let responses = vec![
            Response::Recent {
                matches: vec![RecentMatch {
                    plugin: plugin.clone(),
                    selection,
                    last_used_unix: 123,
                    uses: 2,
                }],
            },
            Response::Health {
                statuses: vec![PluginHealth {
                    plugin: plugin.name.to_string(),
                    state: PluginHealthState::Slow,
                    elapsed_ms: 300,
                }],
            },
        ];

        for response in responses {
            let bytes = bincode::serialize(&response).unwrap();
            let deserialized: Response = bincode::deserialize(&bytes).unwrap();
            assert_eq!(format!("{response:?}"), format!("{deserialized:?}"));
        }
    }

    #[test]
    fn is_ipc_disconnect_detects_direct_unexpected_eof() {
        let e = io::Error::new(io::ErrorKind::UnexpectedEof, "eof");
        assert!(is_ipc_disconnect(&e));
    }

    #[test]
    fn is_ipc_disconnect_detects_direct_connection_reset() {
        let e = io::Error::new(io::ErrorKind::ConnectionReset, "reset");
        assert!(is_ipc_disconnect(&e));
    }

    #[test]
    fn is_ipc_disconnect_rejects_other_errors() {
        let e = io::Error::new(io::ErrorKind::NotFound, "not found");
        assert!(!is_ipc_disconnect(&e));
    }

    #[test]
    fn is_ipc_disconnect_rejects_non_disconnect() {
        let e = io::Error::new(io::ErrorKind::InvalidData, "bad data");
        assert!(!is_ipc_disconnect(&e));
    }
}
