use crate::sys::net as sysnet;
use super::{WebSocket, crypto, frame};
use crate::net::http;

const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub fn upgrade_to_websocket(fd: usize, req: &[u8]) -> Result<WebSocket, &'static str> {
    let key = http::header(req, "Sec-WebSocket-Key").ok_or("no key")?;
    let mut concat = [0u8; 128];
    let key_b = key.as_bytes();
    if key_b.len() + WS_GUID.len() > concat.len() { return Err("key too long"); }
    concat[..key_b.len()].copy_from_slice(key_b);
    concat[key_b.len()..key_b.len()+WS_GUID.len()].copy_from_slice(WS_GUID);

    let mut sha = [0u8; 20];
    crypto::sha1(&concat[..key_b.len()+WS_GUID.len()], &mut sha);

    let mut accept = [0u8; 64];
    let acc_len = crypto::base64_encode(&sha, &mut accept);

    let mut resp = [0u8; 256];
    let pre = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ";
    let tail = b"\r\n\r\n";
    let mut off = 0;
    off += frame::copy(&mut resp[off..], pre);
    off += frame::copy(&mut resp[off..], &accept[..acc_len]);
    off += frame::copy(&mut resp[off..], tail);

    sysnet::send_all(fd, &resp[..off]).map_err(|_| "send")?;
    Ok(WebSocket { fd })
}
