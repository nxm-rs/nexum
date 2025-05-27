use std::collections::HashMap;

use alloy_chains::Chain;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    #[default]
    Disconnected,
}

impl ConnectionState {
    pub fn is_disconnected(&self) -> bool {
        matches!(self, ConnectionState::Disconnected)
    }

    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }
}

impl Serialize for ConnectionState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize `Connected` as `true` and `Disconnected` as `false`
        let as_bool = matches!(self, ConnectionState::Connected);
        serializer.serialize_bool(as_bool)
    }
}

impl<'de> Deserialize<'de> for ConnectionState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize `true` as `Connected` and `false` as `Disconnected`
        let is_connected = bool::deserialize(deserializer)?;
        Ok(if is_connected {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct FrameState {
    pub frame_connected: ConnectionState,
    pub available_chains: HashMap<Chain, ConnectionState>,
    pub current_chain_in_tab: Option<Chain>,
}
