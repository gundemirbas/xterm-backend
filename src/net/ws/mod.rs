mod crypto;
mod frame;
mod handshake;

pub(crate) struct WebSocket {
    pub(crate) fd: usize,
}

pub(crate) use frame::{parse_and_unmask_frames, write_binary_frame};
pub(crate) use handshake::upgrade_to_websocket;
