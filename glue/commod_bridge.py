#!/usr/bin/env python3
"""Bridge between CommunicationMod (stdin/stdout) and a Unix domain socket.

CommunicationMod launches this process. Game state JSON arriving on stdin
is broadcast to all connected socket clients. Commands from any client
are forwarded to CommunicationMod via stdout.
"""

import os
import socket
import threading

SOCKET_PATH = "/tmp/sts_commod.sock"
LOG_FILE = os.path.join(os.path.dirname(__file__), "commod_bridge.log")


def log(msg):
    with open(LOG_FILE, "a") as f:
        f.write(msg + "\n")
        f.flush()


def send_to_game(msg):
    log(f">>> {msg}")
    print(msg, flush=True)


def main():
    log("=== Bridge started ===")

    if os.path.exists(SOCKET_PATH):
        os.unlink(SOCKET_PATH)

    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(SOCKET_PATH)
    server.listen(5)
    log(f"Listening on {SOCKET_PATH}")

    send_to_game("ready")

    clients = []
    clients_lock = threading.Lock()

    def accept_connections():
        while True:
            conn, _ = server.accept()
            log("Client connected")
            with clients_lock:
                clients.append(conn)
            # Start a reader thread for this client
            t = threading.Thread(target=read_from_client, args=(conn,), daemon=True)
            t.start()

    def remove_client(conn):
        with clients_lock:
            if conn in clients:
                clients.remove(conn)
                try:
                    conn.close()
                except Exception:
                    pass
                log("Client disconnected")

    def read_from_client(conn):
        """Read commands from a socket client and forward to game."""
        buf = b""
        while True:
            try:
                chunk = conn.recv(4096)
                if not chunk:
                    remove_client(conn)
                    return
                buf += chunk
                while b"\n" in buf:
                    line, buf = buf.split(b"\n", 1)
                    command = line.decode("utf-8").strip()
                    if command:
                        send_to_game(command)
            except Exception as e:
                log(f"Client read error: {e}")
                remove_client(conn)
                return

    def broadcast(line):
        """Send a message to all connected clients."""
        msg = (line + "\n").encode("utf-8")
        with clients_lock:
            dead = []
            for conn in clients:
                try:
                    conn.sendall(msg)
                except Exception as e:
                    log(f"Client send error: {e}")
                    dead.append(conn)
            for conn in dead:
                clients.remove(conn)
                try:
                    conn.close()
                except Exception:
                    pass

    accept_thread = threading.Thread(target=accept_connections, daemon=True)
    accept_thread.start()

    # Main loop: read game state from stdin, broadcast to all clients
    while True:
        try:
            line = input()
            log(f"<<< {line}")
            broadcast(line)
        except EOFError:
            log("Game closed connection (EOF)")
            break

    log("=== Bridge exiting ===")


if __name__ == "__main__":
    main()
