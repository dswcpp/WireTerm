#!/usr/bin/env python3
"""Verify the native connection helper against an isolated loopback SSH server.

Requires paramiko and, on Windows, pywinpty. All keys and known-host files are temporary.
"""

import argparse
import base64
import hashlib
import json
import logging
import os
from pathlib import Path
import queue
import re
import socket
import subprocess
import tempfile
import threading
import time

import paramiko

logging.getLogger("paramiko.transport").setLevel(logging.CRITICAL)


class Fixture(paramiko.ServerInterface):
    def __init__(self, key):
        self.client_key = key
        self.host_key = paramiko.RSAKey.generate(2048)
        self.events = []
        self.connections = []
        self.running = True
        self.listener = socket.socket()
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen()
        self.listener.settimeout(0.2)
        self.port = self.listener.getsockname()[1]
        self.thread = threading.Thread(target=self.accept, daemon=True)
        self.thread.start()

    def accept(self):
        while self.running:
            try:
                client, _ = self.listener.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            threading.Thread(target=self.serve, args=(client,), daemon=True).start()

    def serve(self, client):
        transport = paramiko.Transport(client)
        self.connections.append(transport)
        transport.add_server_key(self.host_key)
        try:
            transport.start_server(server=self)
            channel = transport.accept(15)
            if channel is None:
                return
            channel.settimeout(0.2)
            ready = False
            line = b""
            while transport.is_active() and self.running:
                if not ready and getattr(channel, "fixture_shell", False):
                    channel.sendall(b"\x1b[32mfixture-ready\x1b[0m\r\n")
                    ready = True
                try:
                    data = channel.recv(4096)
                except socket.timeout:
                    continue
                if not data:
                    break
                if b"\x03" in data:
                    channel.sendall(b"control-c-received\r\n")
                    data = data.replace(b"\x03", b"")
                    line = b""
                line += data
                while b"\r" in line or b"\n" in line:
                    parts = re.split(b"[\r\n]", line, maxsplit=1)
                    command, line = parts
                    if command == b"ping":
                        channel.sendall(b"pong\r\n")
                    elif command == b"exit":
                        channel.send_exit_status(7)
                        return
        except (EOFError, OSError, paramiko.SSHException):
            pass
        finally:
            transport.close()

    def get_allowed_auths(self, username):
        return "publickey,password" if username == "fixture" else "publickey"

    def check_auth_publickey(self, username, key):
        accepted = username == "fixture" and key == self.client_key
        self.events.append({"event": "key-auth", "accepted": accepted})
        return paramiko.AUTH_SUCCESSFUL if accepted else paramiko.AUTH_FAILED

    def check_auth_password(self, username, password):
        accepted = username == "fixture" and password == "fixture-password"
        self.events.append({"event": "password-auth", "accepted": accepted})
        return paramiko.AUTH_SUCCESSFUL if accepted else paramiko.AUTH_FAILED

    def check_channel_request(self, kind, chanid):
        return paramiko.OPEN_SUCCEEDED if kind == "session" else paramiko.OPEN_FAILED_ADMINISTRATIVELY_PROHIBITED

    def check_channel_pty_request(self, channel, term, width, height, pixelwidth, pixelheight, modes):
        self.events.append({"event": "pty", "columns": width, "rows": height})
        return True

    def check_channel_window_change_request(self, channel, width, height, pixelwidth, pixelheight):
        self.events.append({"event": "resize", "columns": width, "rows": height})
        return True

    def check_channel_shell_request(self, channel):
        channel.fixture_shell = True
        return True

    def close(self):
        self.running = False
        self.listener.close()
        for connection in self.connections:
            connection.close()
        self.thread.join(timeout=2)


class Terminal:
    def __init__(self, args):
        if os.name == "nt":
            from winpty import PtyProcess
        else:
            from ptyprocess import PtyProcessUnicode as PtyProcess
        self.process = PtyProcess.spawn(args, dimensions=(24, 80))
        self.chunks = queue.Queue()
        self.output = ""
        self.thread = threading.Thread(target=self.read, daemon=True)
        self.thread.start()

    def read(self):
        try:
            while True:
                data = self.process.read(4096)
                if not data:
                    return
                self.chunks.put(data)
        except (EOFError, OSError):
            pass

    def until(self, expected, timeout=20):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if expected in self.output:
                return
            try:
                self.output += self.chunks.get(timeout=0.1)
            except queue.Empty:
                continue
        raise AssertionError(f"Did not observe {expected!r}; output:\n{self.output}")

    def write(self, text):
        self.process.write(text)

    def finish(self, expected):
        deadline = time.monotonic() + 15
        while self.process.isalive() and time.monotonic() < deadline:
            time.sleep(0.05)
        assert not self.process.isalive(), "SSH helper failed to exit"
        assert self.process.exitstatus == expected, (self.process.exitstatus, self.output)

    def close(self):
        if self.process.isalive():
            self.process.terminate(force=True)


def wait_for(predicate, description):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.05)
    raise AssertionError(description)


def verify(binary, app=False):
    results = []
    prefix = [str(binary)] + (["--wireterm"] if app else [])
    with tempfile.TemporaryDirectory(prefix="wireterm % ssh-") as temp:
        root = Path(temp)
        serial = Terminal(prefix + ["--config-dir", str(root / "serial"), "serial",
                           "--port", "wireterm-test-port-that-does-not-exist"])
        try:
            serial.until("Serial port released")
            serial.finish(1)
        finally:
            serial.close()
        results.append("serial open failure restores terminal mode and releases the connection task")
        key_path = root / "client key"
        client_key = paramiko.RSAKey.generate(2048)
        client_key.write_private_key_file(str(key_path))
        if os.name == "nt":
            script = """
            $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
            $acl = New-Object System.Security.AccessControl.FileSecurity
            $acl.SetOwner($sid)
            $acl.SetAccessRuleProtection($true, $false)
            $rule = [System.Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'Allow')
            $acl.AddAccessRule($rule)
            [System.IO.File]::SetAccessControl($env:WIRETERM_TEST_KEY, $acl)
            """
            permissions = subprocess.run(["powershell", "-NoProfile", "-NonInteractive", "-Command", script],
                                         env={**os.environ, "WIRETERM_TEST_KEY": str(key_path)},
                                         capture_output=True)
            assert permissions.returncode == 0, permissions.stderr.decode(errors="replace")
        else:
            key_path.chmod(0o600)
        fixture = Fixture(client_key)
        terminals = []

        def connect(directory, user="fixture", key=True, via_shell=False):
            args = prefix + ["--config-dir", str(directory), "ssh", "127.0.0.1", "-p", str(fixture.port), "-l", user]
            if key:
                args += ["-i", str(key_path)]
            if via_shell:
                filename = str(binary).replace("'", "''")
                arguments = subprocess.list2cmdline(args[1:]).replace("'", "''")
                command = ("$wireTermConnection = [System.Diagnostics.Process]::Start("
                           "[System.Diagnostics.ProcessStartInfo]@{ FileName = '" + filename +
                           "'; Arguments = '" + arguments + "'; UseShellExecute = $false }); "
                           "$wireTermConnection.WaitForExit(); exit $wireTermConnection.ExitCode")
                args = ["powershell", "-NoProfile", "-Command", command]
            terminal = Terminal(args)
            terminals.append(terminal)
            return terminal

        try:
            denied_root = root / "denied"
            terminal = connect(denied_root)
            terminal.until("Are you sure you want to continue connecting")
            terminal.write("no\r")
            terminal.finish(255)
            assert not (denied_root / "known_hosts").exists()
            assert not fixture.events
            results.append("unknown host key rejection prevents authentication")

            trusted_root = root / "trusted"
            terminal = connect(trusted_root)
            terminal.until("Are you sure you want to continue connecting")
            fingerprint = base64.b64encode(hashlib.sha256(fixture.host_key.asbytes()).digest()).decode().rstrip("=")
            assert fingerprint in terminal.output
            terminal.write("yes\r")
            terminal.until("fixture-ready")
            assert fixture.host_key.get_base64() in (trusted_root / "known_hosts").read_text()
            terminal.process.setwinsize(42, 103)
            wait_for(lambda: any(event == {"event": "resize", "columns": 103, "rows": 42} for event in fixture.events), "SSH resize was not forwarded")
            terminal.write("ping\r")
            terminal.until("pong")
            terminal.write("\x03")
            terminal.until("control-c-received")
            terminal.write("exit\r")
            terminal.finish(7)
            assert any(event["event"] == "key-auth" and event["accepted"] for event in fixture.events)
            results.append("TOFU fingerprint confirmation, key authentication, PTY, resize, interactive IO, Ctrl+C and remote exit status")

            if os.name == "nt":
                terminal = connect(trusted_root, via_shell=True)
                terminal.until("fixture-ready")
                terminal.write("ping\r")
                terminal.until("pong")
                terminal.write("exit\r")
                terminal.finish(7)
                results.append("PowerShell connection launcher inherits console IO and waits for the remote exit status")

            terminal = connect(trusted_root, key=False)
            terminal.until("password:")
            terminal.write("fixture-password\r")
            terminal.until("fixture-ready")
            terminal.write("exit\r")
            terminal.finish(7)
            assert any(event["event"] == "password-auth" and event["accepted"] for event in fixture.events)
            assert "fixture-password" not in terminal.output
            results.append("password authentication without password echo or persistence")

            terminal = connect(trusted_root, user="rejected-user")
            terminal.until("Permission denied")
            terminal.finish(255)
            results.append("authentication failure returns a failed connection")

            fixture.host_key = paramiko.RSAKey.generate(2048)
            before = len(fixture.events)
            terminal = connect(trusted_root)
            terminal.until("REMOTE HOST IDENTIFICATION HAS CHANGED")
            terminal.finish(255)
            assert len(fixture.events) == before
            results.append("changed host key blocks authentication")
        finally:
            for terminal in terminals:
                terminal.close()
            fixture.close()
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--app", action="store_true", help="Verify warp-oss --wireterm instead of the standalone connector")
    args = parser.parse_args()
    results = verify(args.binary.resolve(), app=args.app)
    report = {"platform": os.name, "passed": results}
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
