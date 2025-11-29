#!/usr/bin/env python3
"""
Run all WS tests sequentially:
  1) handshake_raw()    - sends a simple HTTP upgrade handshake and prints server response
  2) handshake_timeout() - same but with a short read timeout to emulate the fd script
  3) ws_client_test()   - performs the masked-frame client test (from test_ws_client.py)

Usage: python3 scripts/run_all_tests.py
"""

import socket
import os
import sys
import time

HOST = '127.0.0.1'
PORT = 8000

REQ = (
    'GET /term HTTP/1.1\r\n'
    'Host: localhost\r\n'
    'Upgrade: websocket\r\n'
    'Connection: Upgrade\r\n'
    'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n'
    'Sec-WebSocket-Version: 13\r\n'
    '\r\n'
)


def handshake_raw():
    print('\n== handshake_raw ==')
    try:
        s = socket.create_connection((HOST, PORT), timeout=5)
        s.sendall(REQ.encode())
        # read until header end or timeout
        s.settimeout(2.0)
        buf = b''
        while True:
            chunk = s.recv(1024)
            if not chunk:
                break
            buf += chunk
            if b'\r\n\r\n' in buf:
                break
        print('HTTP response:')
        print(buf.decode(errors='ignore'))
        s.close()
        return True
    except Exception as e:
        print('handshake_raw failed:', e)
        return False


def handshake_timeout():
    print('\n== handshake_timeout (short read) ==')
    try:
        s = socket.create_connection((HOST, PORT), timeout=5)
        s.sendall(REQ.encode())
        # short non-blocking-ish read to emulate timeout/cat behavior
        s.settimeout(0.8)
        try:
            resp = s.recv(4096)
            print(resp.decode(errors='ignore'))
        except socket.timeout:
            print('(read timed out)')
        s.close()
        return True
    except Exception as e:
        print('handshake_timeout failed:', e)
        return False


def ws_client_test():
    print('\n== ws_client_test ==')
    try:
        s = socket.create_connection((HOST, PORT), timeout=5)
        s.sendall(REQ.encode())

        # read HTTP response headers
        buf = b''
        s.settimeout(2.0)
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
        masked = bytes([payload[i] ^ mask[i % 4] for i in range(len(payload))])
        frame = first + second + mask + masked
        s.sendall(frame)
        print('Sent masked text frame')

        # basic frame parser for a single frame
        s.settimeout(2.0)
        hdr = s.recv(2)
        if not hdr:
            print('no hdr')
            s.close()
            return False
        b0, b1 = hdr[0], hdr[1]
        opcode = b0 & 0x0F
        masked_flag = (b1 & 0x80) != 0
        length = b1 & 0x7F
        if length == 126:
            ext = s.recv(2)
            length = int.from_bytes(ext, 'big')
        elif length == 127:
            ext = s.recv(8)
            length = int.from_bytes(ext, 'big')
        maskkey = None
        if masked_flag:
            maskkey = s.recv(4)
        data = b''
        while len(data) < length:
            chunk = s.recv(length - len(data))
            if not chunk:
                break
            data += chunk
        if maskkey:
            data = bytes([data[i] ^ maskkey[i % 4] for i in range(len(data))])
        print('Received opcode', opcode, 'len', len(data))
        print(data)
        s.close()
        return True
    except Exception as e:
        print('ws_client_test failed:', e)
        return False


if __name__ == '__main__':
    steps = [
        ('handshake_raw', handshake_raw),
        ('handshake_timeout', handshake_timeout),
        ('ws_client_test', ws_client_test),
    ]

    results = {}
    for name, fn in steps:
        print('\n>>> Running', name)
        ok = fn()
        results[name] = ok
        # brief pause between steps
        time.sleep(0.15)

    print('\nSummary:')
    all_ok = True
    for name, ok in results.items():
        print(f'- {name}:', 'OK' if ok else 'FAIL')
        if not ok:
            all_ok = False

    sys.exit(0 if all_ok else 2)
