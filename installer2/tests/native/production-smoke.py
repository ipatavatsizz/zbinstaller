#!/usr/bin/env python3
"""Observe the exact downloaded production artifact; never invoke application actions."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import signal
import struct
import zlib
import subprocess
import sys
import time
import zipfile


def run(command, **kwargs):
    return subprocess.run(command, check=True, capture_output=True, text=True, timeout=20, **kwargs)


def digest(path):
    with path.open("rb") as stream:
        result = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
        return result.hexdigest()


def painted_samples(samples):
    if not samples:
        return False
    brightness = [(299 * r + 587 * g + 114 * b) // 1000 for r, g, b in samples]
    colors = {(r // 32, g // 32, b // 32) for r, g, b in samples}
    return max(brightness) - min(brightness) >= 60 and len(colors) >= 6


def painted_png(path, region=None):
    # Standard-library PNG reader for the native tools' non-interlaced 8-bit RGB(A).
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise RuntimeError("Screenshot is not PNG")
    offset, compressed = 8, bytearray()
    while offset < len(data):
        size = struct.unpack_from(">I", data, offset)[0]
        kind = data[offset + 4:offset + 8]
        chunk = data[offset + 8:offset + 8 + size]
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", chunk)
        elif kind == b"IDAT":
            compressed.extend(chunk)
        offset += size + 12
    if depth != 8 or color not in (2, 6) or compression or filtering or interlace:
        raise RuntimeError("Unsupported native screenshot PNG format")
    channels = 3 if color == 2 else 4
    stride = width * channels
    raw = zlib.decompress(compressed)
    if len(raw) != (stride + 1) * height:
        raise RuntimeError("Invalid PNG scanlines")
    x, y, w, h = region or (0, 0, width, height)
    if not (0 <= x < x + w <= width and 0 <= y < y + h <= height):
        raise RuntimeError("Window bounds outside screenshot")
    # Exclude title bar/borders; an OS-drawn title does not establish WebView paint.
    left, right = x + w // 10, x + w * 9 // 10
    top, bottom = y + h // 10, y + h * 9 // 10
    samples, previous = [], bytearray(stride)
    for row in range(height):
        start = row * (stride + 1)
        mode = raw[start]
        values = bytearray(raw[start + 1:start + 1 + stride])
        for column in range(stride):
            a = values[column - channels] if column >= channels else 0
            b = previous[column]
            c = previous[column - channels] if column >= channels else 0
            if mode == 1:
                predictor = a
            elif mode == 2:
                predictor = b
            elif mode == 3:
                predictor = (a + b) // 2
            elif mode == 4:
                distances = [abs(b - c), abs(a - c), abs(a + b - 2 * c)]
                predictor = (a, b, c)[distances.index(min(distances))]
            elif mode == 0:
                predictor = 0
            else:
                raise RuntimeError("Invalid PNG filter")
            values[column] = (values[column] + predictor) & 255
        if top <= row < bottom and (row - top) % max(1, h // 40) == 0:
            samples.extend(tuple(values[col * channels:col * channels + 3])
                           for col in range(left, right, max(1, w // 40)))
        previous = values
    return painted_samples(samples)


def wait_for_window(process, probe, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"Production process exited early: {process.returncode}")
        windows = probe(process.pid)
        if windows:
            return windows
        time.sleep(0.25)
    raise TimeoutError("No native visible window before startup deadline")


def linux_windows(pid):
    result = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(pid)],
                            capture_output=True, text=True, timeout=5)
    if result.returncode == 1:
        return []
    result.check_returncode()
    windows = []
    for identifier in result.stdout.split():
        title = run(["xdotool", "getwindowname", identifier]).stdout.strip()
        geometry = run(["xdotool", "getwindowgeometry", "--shell", identifier]).stdout
        values = dict(line.split("=", 1) for line in geometry.splitlines() if "=" in line)
        if title.startswith("ZombieBuddy Installer ") and int(values.get("WIDTH", 0)) >= 600 and int(values.get("HEIGHT", 0)) >= 400:
            windows.append({"id": identifier, "pid": pid, "title": title, "geometry": values})
    return windows


SWIFT_PROBE = r"""
import CoreGraphics
import Foundation
let pid = Int(CommandLine.arguments[1])!
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
let windows = list.filter { window in
    let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
    return (window[kCGWindowOwnerPID as String] as? Int) == pid
        && (window[kCGWindowLayer as String] as? Int) == 0
        && (bounds["Width"] as? Double ?? 0) >= 600
        && (bounds["Height"] as? Double ?? 0) >= 400
}
let data = try JSONSerialization.data(withJSONObject: windows, options: [.sortedKeys])
print(String(data: data, encoding: .utf8)!)
"""


def prepare(artifact, output, system):
    if system == "Linux":
        if not artifact.is_file() or artifact.read_bytes()[:4] != b"\x7fELF":
            raise RuntimeError("Expected downloaded Linux ELF executable")
        artifact.chmod(artifact.stat().st_mode | 0o100)
        return artifact, linux_windows
    if system != "Darwin":
        raise RuntimeError("Production smoke requires native Linux or macOS")
    extracted = output / "extracted"
    extracted.mkdir()
    with zipfile.ZipFile(artifact) as archive:
        for name in archive.namelist():
            if Path(name).is_absolute() or ".." in Path(name).parts:
                raise RuntimeError("Unsafe APP ZIP path")
    run(["ditto", "-x", "-k", str(artifact), str(extracted)])
    apps = list(extracted.glob("*.app"))
    if len(apps) != 1:
        raise RuntimeError("Expected exactly one APP in downloaded ZIP")
    with (apps[0] / "Contents/Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    executable = (apps[0] / "Contents/MacOS" / info["CFBundleExecutable"]).resolve(strict=True)
    if not executable.is_relative_to(apps[0].resolve()):
        raise RuntimeError("APP executable escaped bundle")
    (output / "bundle-identity.json").write_text(json.dumps({key: info.get(key) for key in
        ["CFBundleIdentifier", "CFBundleVersion", "CFBundleShortVersionString", "CFBundleExecutable"]}, indent=2))
    source = output / "window-probe.swift"
    source.write_text(SWIFT_PROBE)
    helper = output / "window-probe"
    run(["swiftc", str(source), "-o", str(helper)])
    return executable, lambda pid: json.loads(run([str(helper), str(pid)]).stdout)


def smoke(artifact, output, timeout):
    output.mkdir(parents=True, exist_ok=True)
    report = {"status": "failed", "source_commit": os.environ.get("SOURCE_COMMIT"),
              "runner": platform.platform(), "architecture": platform.machine(),
              "artifact": str(artifact), "artifact_sha256": digest(artifact)}
    process = None
    started = time.monotonic()
    try:
        executable, probe = prepare(artifact, output, platform.system())
        report["executable"] = str(executable)
        report["executable_sha256"] = digest(executable)
        # No WebDriver feature, bridge, injected arguments or executable mutation.
        with (output / "application.log").open("wb") as log:
            process = subprocess.Popen([str(executable)], cwd=executable.parent,
                                       stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            report["pid"] = process.pid
            deadline = time.monotonic() + timeout
            report["windows"] = wait_for_window(process, probe, timeout)
            time.sleep(3)
            screenshot = output / "production-window.png"
            report["paint_attempts"] = []
            while True:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("Visible production WebView remained blank before startup deadline")
                windows = wait_for_window(process, probe, min(5, remaining))
                report["windows_after_settle"] = windows
                region = None
                if platform.system() == "Linux":
                    run(["scrot", "--overwrite", str(screenshot)])
                    geometry = windows[0]["geometry"]
                    region = tuple(int(geometry[key]) for key in ("X", "Y", "WIDTH", "HEIGHT"))
                else:
                    identifier = windows[0]["kCGWindowNumber"]
                    run(["screencapture", "-x", "-o", "-l", str(identifier), str(screenshot)])
                painted = painted_png(screenshot, region)
                report["paint_attempts"].append({"painted": painted, "elapsed": round(time.monotonic() - started, 3)})
                if painted:
                    break
                time.sleep(1)
            if process.poll() is not None:
                raise RuntimeError("Production process exited before acceptance completed")
            if digest(artifact) != report["artifact_sha256"] or digest(executable) != report["executable_sha256"]:
                raise RuntimeError("Production artifact bytes changed")
            report["status"] = "passed"
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        # Own process group only; no name-based kill, Steam/game calls or other app cleanup.
        if process is not None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=5)
            except ProcessLookupError:
                pass
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
            except Exception as error:
                report["status"] = "failed"
                report["cleanup_error"] = str(error)
        report["elapsed_seconds"] = round(time.monotonic() - started, 3)
        (output / "result.json").write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2), flush=True)
    return 0 if report["status"] == "passed" else 1


def self_test():
    import unittest
    from unittest.mock import Mock, patch

    class StartupFailures(unittest.TestCase):
        def test_blank_webview_screenshot_rejected(self):
            import tempfile
            width, height = 80, 60
            rows = []
            for row in range(height):
                # A colorful OS title bar above an otherwise blank WebView must fail.
                pixels = bytes(component for col in range(width) for component in
                               ((col * 3, 255 - col * 3, 40) if row < 5 else (40, 40, 55)))
                rows.append(b"\0" + pixels)
            def chunk(kind, value):
                return struct.pack(">I", len(value)) + kind + value + struct.pack(">I", zlib.crc32(kind + value))
            image = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(b"".join(rows))) + chunk(b"IEND", b"")
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "blank.png"
                path.write_bytes(image)
                self.assertFalse(painted_png(path))
            self.assertTrue(painted_samples([(0, 0, 0), (255, 255, 255), (0, 255, 0), (255, 0, 0), (0, 0, 255), (128, 128, 128)]))

        def test_early_exit_rejected(self):
            process = Mock(pid=123, returncode=9)
            process.poll.return_value = 9
            with self.assertRaisesRegex(RuntimeError, "exited early"):
                wait_for_window(process, lambda _: [{"id": 1}], 1)

        def test_missing_window_rejected(self):
            process = Mock(pid=123)
            process.poll.return_value = None
            with self.assertRaises(TimeoutError):
                wait_for_window(process, lambda _: [], 0)

        def test_visible_pid_is_required(self):
            process = Mock(pid=123)
            process.poll.return_value = None
            with patch("time.sleep"):
                self.assertEqual(wait_for_window(process, lambda pid: [{"pid": pid}], 1), [{"pid": 123}])

    return 0 if unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(StartupFailures)).wasSuccessful() else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--timeout", type=float, default=40)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        sys.exit(self_test())
    if not args.artifact or not args.output or not 0 < args.timeout <= 90:
        parser.error("--artifact, --output and timeout in (0, 90] required")
    sys.exit(smoke(args.artifact.resolve(strict=True), args.output.resolve(), args.timeout))
