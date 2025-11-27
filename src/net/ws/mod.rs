pub mod crypto;
pub mod frame;
pub mod handshake;

pub struct WebSocket {
    pub fd: usize,
}

pub use frame::{parse_and_unmask_frames, write_binary_frame};
pub use handshake::upgrade_to_websocket;
