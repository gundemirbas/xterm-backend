pub(crate) fn is_websocket_upgrade(req: &[u8]) -> bool {
    let upgrade = header(req, "Upgrade");
    let connection = header(req, "Connection");

    match (upgrade, connection) {
        (Some(u), Some(c)) => {
            eq_case_insensitive(u, "websocket")
                && c.split(',')
                    .any(|part| eq_case_insensitive(part.trim(), "upgrade"))
        }
        _ => false,
    }
}

pub(crate) fn path_is_term(req: &[u8]) -> bool {
    if let Some(line) = first_line(req) {
        line.starts_with("GET /term ")
    } else {
        false
    }
}

fn first_line(req: &[u8]) -> Option<&str> {
    let s = core::str::from_utf8(req).ok()?;
    let mut it = s.split("\r\n");
    it.next()
}

pub(crate) fn header<'a>(req: &'a [u8], name: &str) -> Option<&'a str> {
    let s = core::str::from_utf8(req).ok()?;
    for line in s.split("\r\n").skip(1) {
        if line.is_empty() {
            break;
        }
        if let Some(pos) = line.find(':') {
            let (k, v) = line.split_at(pos);
            if eq_case_insensitive(k.trim(), name) {
                return Some(v[1..].trim());
            }
        }
    }
    None
}

fn eq_case_insensitive(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.chars()
        .zip(b.chars())
        .all(|(x, y)| x.eq_ignore_ascii_case(&y))
}

pub(crate) fn serve_html(fd: usize, body: &[u8]) {
    let head = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: ";
    let mut lenbuf = itoa::Buffer::new();
    let len_str = lenbuf.format(body.len() as u64);
    let tail = b"\r\nConnection: close\r\n\r\n";
    let _ = crate::sys::net::send_all(fd, head);
    let _ = crate::sys::net::send_all(fd, len_str.as_bytes());
    let _ = crate::sys::net::send_all(fd, tail);
    let _ = crate::sys::net::send_all(fd, body);
    let _ = crate::sys::fs::close(fd);
}
