use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct FrameState {
    pub frame_connected: bool,
    pub available_chains: Vec<String>,
    pub current_chain: Option<u32>,
}
