pub fn sha1(msg: &[u8], out: &mut [u8; 20]) {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let ml = (msg.len() as u64) * 8;
    let mut i = 0;
    let mut block = [0u8; 64];
    while i < msg.len() {
        let mut bi = 0;
        while bi < 64 && i < msg.len() {
            block[bi] = msg[i];
            bi += 1;
            i += 1;
        }
        if bi < 64 {
            block[bi] = 0x80;
            bi += 1;
            if bi > 56 {
                for k in bi..64 {
                    block[k] = 0;
                }
                sha1_block(&mut h0, &mut h1, &mut h2, &mut h3, &mut h4, &block);
                for k in 0..56 {
                    block[k] = 0;
                }
            } else {
                for k in bi..56 {
                    block[k] = 0;
                }
            }
            for k in 0..8 {
                block[56 + k] = ((ml >> (8 * (7 - k))) & 0xFF) as u8;
            }
            sha1_block(&mut h0, &mut h1, &mut h2, &mut h3, &mut h4, &block);
            break;
        } else {
            if i == msg.len() {
                for k in 0..64 {
                    block[k] = 0;
                }
                block[0] = 0x80;
                for k in 0..8 {
                    block[56 + k] = ((ml >> (8 * (7 - k))) & 0xFF) as u8;
                }
                sha1_block(&mut h0, &mut h1, &mut h2, &mut h3, &mut h4, &block);
                break;
            } else {
                sha1_block(&mut h0, &mut h1, &mut h2, &mut h3, &mut h4, &block);
            }
        }
    }
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
}

fn sha1_block(
    h0: &mut u32,
    h1: &mut u32,
    h2: &mut u32,
    h3: &mut u32,
    h4: &mut u32,
    block: &[u8; 64],
) {
    let mut w = [0u32; 80];
    for i in 0..16 {
        w[i] = ((block[i * 4] as u32) << 24)
            | ((block[i * 4 + 1] as u32) << 16)
            | ((block[i * 4 + 2] as u32) << 8)
            | (block[i * 4 + 3] as u32);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let mut a = *h0;
    let mut b = *h1;
    let mut c = *h2;
    let mut d = *h3;
    let mut e = *h4;

    for i in 0..80 {
        let (f, k) = if i < 20 {
            ((b & c) | ((!b) & d), 0x5A827999)
        } else if i < 40 {
            (b ^ c ^ d, 0x6ED9EBA1)
        } else if i < 60 {
            ((b & c) | (b & d) | (c & d), 0x8F1BBCDC)
        } else {
            (b ^ c ^ d, 0xCA62C1D6)
        };
        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(w[i]);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;
    }
    *h0 = h0.wrapping_add(a);
    *h1 = h1.wrapping_add(b);
    *h2 = h2.wrapping_add(c);
    *h3 = h3.wrapping_add(d);
    *h4 = h4.wrapping_add(e);
}

pub fn base64_encode(src: &[u8], dst: &mut [u8]) -> usize {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut i = 0;
    let mut o = 0;
    while i + 3 <= src.len() {
        let v = ((src[i] as u32) << 16) | ((src[i + 1] as u32) << 8) | (src[i + 2] as u32);
        dst[o] = T[((v >> 18) & 63) as usize];
        dst[o + 1] = T[((v >> 12) & 63) as usize];
        dst[o + 2] = T[((v >> 6) & 63) as usize];
        dst[o + 3] = T[(v & 63) as usize];
        i += 3;
        o += 4;
    }
    let rem = src.len() - i;
    if rem == 1 {
        let v = (src[i] as u32) << 16;
        dst[o] = T[((v >> 18) & 63) as usize];
        dst[o + 1] = T[((v >> 12) & 63) as usize];
        dst[o + 2] = b'=';
        dst[o + 3] = b'=';
        o += 4;
    } else if rem == 2 {
        let v = ((src[i] as u32) << 16) | ((src[i + 1] as u32) << 8);
        dst[o] = T[((v >> 18) & 63) as usize];
        dst[o + 1] = T[((v >> 12) & 63) as usize];
        dst[o + 2] = T[((v >> 6) & 63) as usize];
        dst[o + 3] = b'=';
        o += 4;
    }
    o
}
