"""Bounded USB observation; optional local controls or acoustic cue test.

uv run --with pyserial python scripts/observe-device.py --seconds 30
--commands t:2,s:4,s:9 sends t after 2 s, status at 4 and 9 s.
--say time uses the Mac speaker; this is a physical acoustic test, not an injected event.
"""
import argparse
import subprocess
import time
import serial

parser = argparse.ArgumentParser()
parser.add_argument("--port", default="/dev/cu.usbmodem2101")
parser.add_argument("--seconds", type=float, default=20)
parser.add_argument("--commands", default="")
parser.add_argument("--say")
parser.add_argument("--audio-device", help="macOS say output device ID (say -a '?')")
args = parser.parse_args()
commands = []
for pair in filter(None, args.commands.split(",")):
    char, delay = pair.split(":")
    commands.append((float(delay), char.encode()))
commands.sort()
port = serial.Serial()
port.port, port.baudrate, port.timeout = args.port, 115200, 0.1
port.dtr = port.rts = False
port.open()
start = time.monotonic()
spoken = False
while time.monotonic() - start < args.seconds:
    elapsed = time.monotonic() - start
    while commands and elapsed >= commands[0][0]:
        _, command = commands.pop(0)
        port.write(command)
    if args.say and not spoken and elapsed >= 4:
        print(f"ACOUSTIC TEST: {args.say}", flush=True)
        command = ["say", "-v", "Samantha"]
        if args.audio_device: command += ["-a", args.audio_device]
        subprocess.Popen(command + [args.say])
        spoken = True
    data = port.read(4096)
    if data:
        print(data.decode(errors="replace"), end="", flush=True)
port.close()
