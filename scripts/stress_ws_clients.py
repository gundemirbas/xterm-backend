#!/usr/bin/env python3
"""
Stress test: open many websocket connections to /term to exercise worker limit.
"""
import socket, threading, time, os
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

N=16
sockets=[]

def send_keepalive(sock, rounds=10, interval=0.5):
    # send small masked text frames periodically to keep connection alive
    try:
        for _ in range(rounds):
            payload = b'\n'
            mask = os.urandom(4)
            first = bytes([0x81])
            second = bytes([0x80 | len(payload)])
            masked = bytes([payload[i] ^ mask[i%4] for i in range(len(payload))])
            frame = first + second + mask + masked
            sock.send(frame)
            time.sleep(interval)
    except Exception:
        pass


def client(i):
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
        # start keepalive sender thread for this socket
        t = threading.Thread(target=send_keepalive, args=(s,10,0.5), daemon=True)
        t.start()
        sockets.append(s)
    except Exception as e:
        print(f"client {i} error: {e}")

ths=[]
for i in range(N):
    t=threading.Thread(target=client,args=(i,))
    t.start()
    ths.append(t)
    time.sleep(0.05)

for t in ths:
    t.join()

print('opened', len(sockets), 'sockets')
print('sleeping 8s to observe server logs (keepalives running)')
time.sleep(8)
for s in sockets:
    try:
        s.close()
    except:
        pass
print('done')
