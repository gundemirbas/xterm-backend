pub mod handshake;
pub mod frame;
pub mod crypto;

pub struct WebSocket {
    pub fd: usize,
}

pub use handshake::upgrade_to_websocket;
pub use frame::{write_binary_frame, parse_and_unmask_frames};
