use crate::sys::net as sysnet;

pub fn write_binary_frame(fd: usize, payload: &[u8]) -> Result<(), &'static str> {
    let mut hdr = [0u8; 10];
    hdr[0] = 0x80 | 0x2; // FIN + binary
    let off = if payload.len() < 126 {
        hdr[1] = payload.len() as u8;
        2
    } else if payload.len() <= 0xFFFF {
        hdr[1] = 126;
        hdr[2] = ((payload.len() >> 8) & 0xFF) as u8;
        hdr[3] = (payload.len() & 0xFF) as u8;
        4
    } else {
        hdr[1] = 127;
        for i in 0..8 {
            hdr[2 + i] = ((payload.len() as u64 >> (8 * (7 - i))) & 0xFF) as u8;
        }
        10
    };
    sysnet::send_all(fd, &hdr[..off]).map_err(|_| "send hdr")?;
    sysnet::send_all(fd, payload).map_err(|_| "send payload")
}

pub fn parse_and_unmask_frames<'a>(
    input: &[u8],
    out: &'a mut [u8],
) -> Result<&'a [u8], &'static str> {
    if input.len() < 2 {
        return Err("short");
    }
    let b0 = input[0];
    let b1 = input[1];
    // diagnostic logging of header bytes
    let _ = crate::sys::fs::write(1, b"ws: frame hdr: ");
    let mut tmp = [0u8; 32];
    let mut ti = 0usize;
    tmp[ti] = b0; ti += 1;
    tmp[ti] = b' '; ti += 1;
    tmp[ti] = b1; ti += 1;
    tmp[ti] = b'\n'; ti += 1;
    let _ = crate::sys::fs::write(1, &tmp[..ti]);
    let masked = (b1 & 0x80) != 0;
    let mut idx = 2;
    let mut len: usize = (b1 & 0x7F) as usize;
    if len == 126 {
        if input.len() < idx + 2 {
            return Err("short");
        }
        len = ((input[idx] as usize) << 8) | (input[idx + 1] as usize);
        idx += 2;
    } else if len == 127 {
        if input.len() < idx + 8 {
            return Err("short");
        }
        let mut l: u64 = 0;
        for i in 0..8 {
            l = (l << 8) | (input[idx + i] as u64);
        }
        idx += 8;
        len = l as usize;
    }
    if !masked {
        return Err("client not masked");
    }
    if input.len() < idx + 4 + len {
        return Err("short");
    }
    let key = &input[idx..idx + 4];
    idx += 4;
    for i in 0..len {
        out[i] = input[idx + i] ^ key[i % 4];
    }
    let opcode = b0 & 0x0F;
    if opcode == 0x8 {
        return Err("close");
    }
    Ok(&out[..len])
}

pub fn copy(dst: &mut [u8], src: &[u8]) -> usize {
    let n = core::cmp::min(dst.len(), src.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}
