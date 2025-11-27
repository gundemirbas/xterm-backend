#!/usr/bin/env python3
"""
Open MAX_WORKERS connections, close them, then try one more to verify server accepts new connections after reaping.
"""
import socket, time
HOST='127.0.0.1'
PORT=8000
REQ=(
    'GET /term HTTP/1.1\r\n'
    'Host: localhost\r\n'
    'Upgrade: websocket\r\n'
    'Connection: Upgrade\r\n'
    'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n'
    'Sec-WebSocket-Version: 13\r\n'
    '\r\n'
)

MAX=15
sockets=[]
for i in range(MAX):
    try:
        s=socket.create_connection((HOST,PORT),timeout=5)
        s.send(REQ.encode())
        buf=b''
        while b'\r\n\r\n' not in buf:
            part=s.recv(1024)
            if not part:
                break
            buf+=part
        print(f"client {i}: got reply len={len(buf)}")
        sockets.append(s)
    except Exception as e:
        print(f"client {i} error: {e}")

print('opened', len(sockets))
print('closing sockets')
for s in sockets:
    try: s.close()
    except: pass

print('sleep 1s to allow reaping')
time.sleep(1)

print('attempting a new connection')
try:
    s=socket.create_connection((HOST,PORT),timeout=5)
    s.send(REQ.encode())
    buf=b''
    while b'\r\n\r\n' not in buf:
        part=s.recv(1024)
        if not part:
            break
        buf+=part
    print('new conn reply len=', len(buf))
    s.close()
except Exception as e:
    print('new conn failed:', e)
