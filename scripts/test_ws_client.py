#!/usr/bin/env python3
"""
Minimal WebSocket client to test masked frames against local server.
Sends handshake, then sends a masked text frame containing 'ls\n', then reads frames from server.
"""
import socket
import base64
import hashlib
import os

HOST='127.0.0.1'
PORT=8000

req = (
    'GET /term HTTP/1.1\r\n'
    'Host: localhost\r\n'
    'Upgrade: websocket\r\n'
    'Connection: Upgrade\r\n'
    'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n'
    'Sec-WebSocket-Version: 13\r\n'
    '\r\n'
)

s = socket.create_connection((HOST, PORT), timeout=5)
s.send(req.encode())

# read HTTP response headers
buf = b''
while True:
    data = s.recv(1024)
    if not data:
        break
    buf += data
    if b'\r\n\r\n' in buf:
        break
print('HTTP response:')
print(buf.decode(errors='ignore'))

# send masked text frame with payload 'ls\n'
payload = b'ls\n'
mask = os.urandom(4)
first = bytes([0x81])
second = bytes([0x80 | len(payload)])
masked = bytes([payload[i] ^ mask[i%4] for i in range(len(payload))])
frame = first + second + mask + masked
s.send(frame)
print('Sent masked text frame')

# read binary frames from server (basic parser)
try:
    hdr = s.recv(2)
    if not hdr:
        print('no hdr')
    else:
        b0, b1 = hdr[0], hdr[1]
        fin = (b0 & 0x80) != 0
        opcode = b0 & 0x0F
        masked = (b1 & 0x80) != 0
        length = b1 & 0x7F
        if length == 126:
            ext = s.recv(2)
            length = int.from_bytes(ext, 'big')
        elif length == 127:
            ext = s.recv(8)
            length = int.from_bytes(ext, 'big')
        maskkey = None
        if masked:
            maskkey = s.recv(4)
        data = b''
        while len(data) < length:
            chunk = s.recv(length - len(data))
            if not chunk:
                break
            data += chunk
        if maskkey:
            data = bytes([data[i] ^ maskkey[i%4] for i in range(len(data))])
        print('Received opcode', opcode, 'len', len(data))
        print(data)
except Exception as e:
    print('error reading frame', e)

s.close()
