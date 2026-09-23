"""Optional macOS acoustic regression check. Generates local TTS; does not play audio.

Build `cargo +esp build -p hmi-core --example sound-template` first.
Generated recordings stay in ignored artifacts/, never on the device.
"""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
artifacts = root / "artifacts"
cases = [
    ("time", "Samantha", 220, True),
    ("time", "Daniel", 140, True),
    ("hello", "Samantha", 175, False),
    ("weather", "Daniel", 175, False),
    ("coffee", "Samantha", 175, False),
    ("banana", "Daniel", 175, False),
    ("stop", "Samantha", 175, False),
]
failed = []
for index, (word, voice, rate, expected) in enumerate(cases):
    aiff = artifacts / f"cue-{index}.aiff"
    pcm = artifacts / f"cue-{index}.pcm"
    subprocess.run(["say", "-v", voice, "-r", str(rate), "-o", str(aiff), word], check=True)
    subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(aiff),
                    "-ar", "24000", "-ac", "1", "-f", "s16le", str(pcm)], check=True)
    result = subprocess.run([str(root / "target/debug/examples/sound-template"), "detect", str(pcm)],
                            check=True, capture_output=True, text=True)
    detected = "Time" in result.stderr
    print(f"{word:8} {voice:9} rate={rate}: detected={detected}, {result.stdout.strip()}")
    if detected != expected:
        failed.append(word)
assert not failed, f"Unexpected classifications: {failed}"
