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
    ReloadPlugins,
    /// Reload a specific plugin by name.
    /// Useful when a plugin's config file changes.
    ReloadPlugin {
        /// The plugin name (e.g. "calc" for "calc.ron")
        name: String,
    },
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
        bincode::deserialize::<T>(&buf).map_err(io::Error::other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reload_plugins_request_serializes() {
        let req = Request::ReloadPlugins;
        let bytes = bincode::serialize(&req).unwrap();
        let deserialized: Request = bincode::deserialize(&bytes).unwrap();
        assert!(matches!(deserialized, Request::ReloadPlugins));
    }

    #[test]
    fn test_all_request_variants_roundtrip() {
        let requests = vec![
            Request::Reset,
            Request::Query {
                text: "test".into(),
                phase: QueryPhase::Typing,
                plugins: vec!["Applications".into()],
            },
            Request::Quit,
            Request::ReloadPlugins,
        ];
        for req in requests {
            let bytes = bincode::serialize(&req).unwrap();
            let deserialized: Request = bincode::deserialize(&bytes).unwrap();
            assert_eq!(format!("{req:?}"), format!("{deserialized:?}"));
        }
    }
}
