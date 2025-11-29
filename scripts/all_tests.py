#!/usr/bin/env python3
"""
Combined test runner for xterm-backend WebSocket tests.

Provides the following tests as Python functions so they can be run
individually or all together:
 - handshake_raw
 - handshake_timeout (fd-style)
 - ws_client_test
 - stress_clients
 - reclaim_workers
 - graceful_shutdown

Usage: python3 scripts/all_tests.py [all|handshake_raw|handshake_timeout|ws_client_test|stress|reclaim|graceful]
"""
import os
import socket
import sys
import time
import threading
import subprocess

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


def handshake_raw(timeout=5.0):
    print('\n== handshake_raw ==')
    try:
        s = socket.create_connection((HOST, PORT), timeout=timeout)
        s.sendall(REQ.encode())
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
    print('\n== handshake_timeout (fd-style) ==')
    try:
        s = socket.create_connection((HOST, PORT), timeout=5)
        s.sendall(REQ.encode())
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
    import os as _os
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
        mask = _os.urandom(4)
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


def stress_clients(n=16):
    print('\n== stress_clients ==')
    import os as _os
    import threading as _threading
    import time as _time

    REQ_LOCAL = REQ
    sockets = []

    def send_keepalive(sock, rounds=10, interval=0.5):
        try:
            for _ in range(rounds):
                payload = b'\n'
                mask = _os.urandom(4)
                first = bytes([0x81])
                second = bytes([0x80 | len(payload)])
                masked = bytes([payload[i] ^ mask[i % 4] for i in range(len(payload))])
                frame = first + second + mask + masked
                sock.send(frame)
                _time.sleep(interval)
        except Exception:
            pass

    def client(i):
        try:
            s = socket.create_connection((HOST, PORT), timeout=5)
            s.send(REQ_LOCAL.encode())
            buf = b''
            while b'\r\n\r\n' not in buf:
                part = s.recv(1024)
                if not part:
                    break
                buf += part
            print(f"client {i}: got reply len={len(buf)}")
            t = _threading.Thread(target=send_keepalive, args=(s, 10, 0.5), daemon=True)
            t.start()
            sockets.append(s)
        except Exception as e:
            print(f"client {i} error: {e}")

    ths = []
    for i in range(n):
        t = threading.Thread(target=client, args=(i,))
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
        except Exception:
            pass
    print('done')


def reclaim_workers(max_workers=15):
    print('\n== reclaim_workers ==')
    sockets = []
    for i in range(max_workers):
        try:
            s = socket.create_connection((HOST, PORT), timeout=5)
            s.send(REQ.encode())
            buf = b''
            while b'\r\n\r\n' not in buf:
                part = s.recv(1024)
                if not part:
                    break
                buf += part
            print(f"client {i}: got reply len={len(buf)}")
            sockets.append(s)
        except Exception as e:
            print(f"client {i} error: {e}")

    print('opened', len(sockets))
    print('closing sockets')
    for s in sockets:
        try:
            s.close()
        except Exception:
            pass

    print('sleep 1s to allow reaping')
    time.sleep(1)

    print('attempting a new connection')
    try:
        s = socket.create_connection((HOST, PORT), timeout=5)
        s.send(REQ.encode())
        buf = b''
        while b'\r\n\r\n' not in buf:
            part = s.recv(1024)
            if not part:
                break
            buf += part
        print('new conn reply len=', len(buf))
        s.close()
    except Exception as e:
        print('new conn failed:', e)


def _find_listening_pid(port=8000):
    # Try ss, fallback to lsof
    try:
        out = subprocess.check_output(['ss', '-ltnp'], stderr=subprocess.DEVNULL).decode(errors='ignore')
        for line in out.splitlines():
            if f':{port} ' in line or f':{port}\n' in line or f':{port}:' in line:
                # parse pid=NNNN,
                import re
                m = re.search(r'pid=(\d+),', line)
                if m:
                    return int(m.group(1))
    except Exception:
        pass
    try:
        out = subprocess.check_output(['lsof', '-tiTCP:%d' % port, '-sTCP:LISTEN'], stderr=subprocess.DEVNULL).decode(errors='ignore')
        if out.strip():
            return int(out.splitlines()[0].strip())
    except Exception:
        pass
    return None


def graceful_shutdown_test():
    print('\n== graceful_shutdown ==')
    script_dir = os.path.dirname(__file__)
    root = os.path.abspath(os.path.join(script_dir, '..'))
    server_bin = os.path.join(root, 'target', 'x86_64-unknown-linux-gnu', 'release', 'xterm-backend')
    server_log = os.path.join(root, 'server.log')

    # Start server
    print('Starting server...')
    logf = open(server_log, 'wb')
    server_proc = subprocess.Popen([server_bin], cwd=root, stdout=logf, stderr=logf)
    time.sleep(0.5)

    # detect listening pid
    listen_pid = _find_listening_pid(PORT)
    if listen_pid:
        print('server pid (listening):', listen_pid)
    else:
        print('No listening PID detected, using started PID', server_proc.pid)
        listen_pid = server_proc.pid

    # start a client in background to hold session
    client_t = threading.Thread(target=ws_client_test, daemon=True)
    client_t.start()
    time.sleep(0.5)

    print('--- server.log (head) ---')
    try:
        with open(server_log, 'rb') as f:
            data = f.read(4096)
            print(data.decode(errors='ignore'))
    except Exception as e:
        print('read server.log head failed:', e)

    print('Killing server with SIGTERM')
    try:
        os.kill(listen_pid, 15)
    except Exception:
        try:
            server_proc.terminate()
        except Exception:
            pass
    time.sleep(1)

    print('--- server.log (tail) ---')
    try:
        with open(server_log, 'rb') as f:
            f.seek(0, os.SEEK_END)
            size = f.tell()
            f.seek(max(0, size - 4096))
            print(f.read().decode(errors='ignore'))
    except Exception as e:
        print('read server.log tail failed:', e)

    print('Processes after SIGTERM:')
    try:
        out = subprocess.check_output(['ps', 'aux'], stderr=subprocess.DEVNULL).decode(errors='ignore')
        for line in out.splitlines():
            if 'xterm-backend' in line or '/bin/sh' in line:
                print(line)
    except Exception:
        pass

    print('Cleaning up')
    try:
        client_t.join(timeout=0.1)
    except Exception:
        pass
    try:
        server_proc.kill()
    except Exception:
        pass
    try:
        logf.close()
    except Exception:
        pass


def main():
    mapping = {
        'handshake_raw': handshake_raw,
        'handshake_timeout': handshake_timeout,
        'ws_client_test': ws_client_test,
        'stress': lambda: stress_clients(16),
        'reclaim': reclaim_workers,
        'graceful': graceful_shutdown_test,
        'all': None,
    }

    args = sys.argv[1:]
    if not args:
        args = ['all']

    if 'all' in args:
        steps = ['handshake_raw', 'handshake_timeout', 'ws_client_test', 'stress', 'reclaim', 'graceful']
    else:
        steps = args

    results = {}
    for name in steps:
        fn = mapping.get(name)
        if fn is None and name != 'graceful':
            print('Unknown step', name)
            results[name] = False
            continue
        print('\n>>> Running', name)
        try:
            ok = fn() if fn is not None else True
        except Exception as e:
            print('step raised exception:', e)
            ok = False
        results[name] = ok
        time.sleep(0.15)

    print('\nSummary:')
    all_ok = True
    for name, ok in results.items():
        print(f'- {name}:', 'OK' if ok else 'FAIL')
        if not ok:
            all_ok = False
    sys.exit(0 if all_ok else 2)


if __name__ == '__main__':
    main()
