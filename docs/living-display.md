# Living Display

Fork: https://github.com/pankajgarkoti/hmi-esp

## Outcome and scope

A landscape 400x300 living display for the Waveshare ESP32-S3-RLCD-4.2, with
independent remembered automata, microphone-driven randomness, and brief clock
glances. User-authorized scope includes implementation, physical-board flashing,
and publishing source to the fork. The user requested a bounded, interesting
result rather than indefinite feature expansion.

The simulation grid is **200 × 124 cells** inside the 400 × 300 landscape
interface, with two display pixels per cell. This has four times the cells
of the original 100 × 62 grid. On first boot after upgrading, checksummed old
worlds are centered in the larger grid; their cells and generation counters
are kept, and the outer area gets a sparse, deterministic starting pattern.
The first new SD save keeps the preceding snapshot both as a recovery copy
and as a permanent `old-XX.bin` file for a possible rollback. Later
saves rotate only the normal `.bin`/`.bak` pair. If both current snapshots are
corrupt, startup falls back to that older archive. Existing Wi-Fi profiles and
device settings are unaffected by the grid migration.

## Controls and defaults

| Action | Result |
|---|---|
| KEY click | Five-second clock glance, or return from clock/settings |
| KEY hold | Return to simulation |
| BOOT click | Open Settings, then advance selection |
| BOOT hold | Change selected setting |
| PWR hold | Hardware power off |
| Finger snap / matching "time" sound | Temporary clock; animation keeps evolving |
| Settings > WI-FI SETUP > BOOT hold | Temporary WPA2 setup network with instructions on screen |

The board now ignores unrelated USB traffic. Diagnostic commands must be a whole
line such as `@LIVING\tSTATUS\n`; unframed serial bytes from other programs cannot
open Settings or the clock. A separate incompatible AgentDeck USB companion may
still reset the USB peripheral on reconnect if it is started against this board;
leave that companion in simulator mode or stopped when using Living Display.

The board samples the battery ADC periodically and shows an **approximate voltage
percentage** at the top-right of the home field and clock. It is not a calibrated
battery fuel gauge.

### On-device Wi-Fi setup

1. Click BOOT from the field to open Settings. Click BOOT seven more times to
   select **WI-FI SETUP**; hold BOOT to start it.
2. With a phone, join `Living-RLCD` using the 12-character temporary password
   shown on the board. If the phone warns the network has no internet, choose to
   stay connected. Open the **HTTP address shown on the board** in the browser
   (typically `http://192.168.71.1`; the actual AP address is read at runtime).
3. Enter the desired 2.4 GHz Wi-Fi SSID and password. The temporary AP closes
   after submission; the board tries the new network and saves its credentials
   in NVS only after obtaining an address. Previous profiles are retained as
   fallback choices. If joining fails, the previous network is restored.
   Hold/select the setup row again to retry.

For a build with two local fallback networks, set `WIFI_SSID`/`WIFI_PASSWORD`
and optional `WIFI_EXTRA_SSID`/`WIFI_EXTRA_PASSWORD` in the ignored `.env.local`.
The optional extra is attempted first; on a failed connection the device rotates
through the other saved and compiled networks every 15 seconds. A successfully
provisioned network takes precedence. Credentials are embedded in a local build
and NVS, never in the public source tree.

The temporary WPA2 AP is enabled only on explicit physical setup and ends after
three minutes or a KEY press. The HTTP form is served only in AP mode and uses no
internet or remote service. Do not enter credentials from an untrusted phone.
Previously saved automata and settings are not deleted by reconfiguration.

Default speed is **4 generations/second**. Options are 2, 4, 8 and 16.
Clock duration options are 3, 5 and 8 seconds. Repeated cues cannot pin the clock
indefinitely: retriggering is suppressed until 1.5 seconds after the overlay ends.

Microphone **Off / Subtle / Responsive / Wild** controls live-cell injection, not
the clock detector. Raw microphone samples mix into a small PRNG; louder samples
increase injection. Off preserves exact cellular rules after the initial seed.
New Seed deliberately resets only the current grid and its generation counter.

Settings persist in NVS. "Teach Time" captures the next short utterance within ten
seconds and saves its compact spectral fingerprint in NVS, not its audio.

## A small collection of very different worlds

| World | Rule | What to watch |
|---|---|---|
| Conway | B3/S23 | Gliders, oscillators, quiet islands |
| HighLife | B36/S23 | Self-replicating seeds |
| Brian's Brain | B2/S/3 | Firing cells and dotted refractory trails |
| Seeds | B2/S | Explosive, short-lived sparks |
| Day & Night | B3678/S34678 | Symmetric behaviour of light and dark |
| Replicator | B1357/S1357 | Parity-driven fractal copying |
| Life Without Death | B3/S012345678 | Irreversible ink growth |
| Diamoeba | B35678/S5678 | Large islands with fluctuating boundaries |
| Maze | B3/S12345 | Growing corridors |
| Anneal | B4678/S35678 | Smoothing boundaries |
| 2 x 2 | B36/S125 | Block-structured evolution |
| Morley | B368/S245 | Complex moving forms |
| Coral | B3/S45678 | Branching growth |
| Coagulations | B378/S235678 | Dense clusters |
| Assimilation | B345/S4567 | Merging structures |
| Vote | B5678/S45678 | Local-majority domains |

The short research pass informed distinct seed densities rather than treating every
rule as the same random soup. New worlds/reseeds sometimes use Acorn, a HighLife
replicator, or sparse kernels; 2 x 2 starts with aligned blocks. HighLife's 12-cell
replicator doubling after 12 generations is a regression test. Microphone
perturbations can disrupt these engineered patterns; switch them off to observe
the unmodified behaviour. The grid is a finite torus, so textbook infinite-plane
lifetimes are not promised.

Sources consulted:
- [Life-like cellular automata](https://en.wikipedia.org/wiki/Life-like_cellular_automaton):
  rules, distinct behaviours, 2 x 2 block property, and Brian's Brain's extra state.
- [HighLife](https://en.wikipedia.org/wiki/Highlife_(cellular_automaton)):
  replication after twelve generations.
- [Conway's Game of Life](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life):
  methuselah seeds and finite-grid boundary effects.

Optional Auto Tour visits a different world every 30 seconds, two minutes or five
minutes. Each world resumes its own grid and generation. Inactive worlds are parked;
the **active** simulation continues behind clock and settings screens.

## Architecture and failure boundaries

- ST7305 physical packing stays 300x400; a tested draw target rotates the logical
  image to landscape. Panel initialization and SPI pins are unchanged.
- Pure Rust simulation uses two bounded grids. Simulation stepping is independent
  of rendering and overlay deadlines. No neural speech models or extra partitions.
- Nonblocking Wi-Fi connection/reconnection leaves simulation usable offline.
- A tiny acoustic detector averages 24kHz input to 8kHz and extracts eight spectral
  bands every 20ms. Dynamic time warping compares short sounds against reference
  and personally learned "time" fingerprints. Snap detection checks attack,
  broadband energy, crest factor and short duration. **This is sound matching, not
  word-level transcription**; similar sounds can false-trigger. Personal calibration
  improves matching. No guarantee across all voices, rooms or microphone distances.
- Independent RAM worlds prevent resetting when changing rules. An SD worker writes
  versioned, checksummed snapshots via a synced temporary file and retained backup.
  Save on switches and every 30 seconds; unchanged worlds are skipped. Data writes
  run outside the simulation loop. No-SD mode retains worlds only until power-off.
- Invalid primary snapshots fall back to the previous copy; corrupt dimensions,
  checksums, rule IDs and cell states are rejected. No SD formatting occurs.
- Audio is processed in RAM and not recorded. Existing Touch349 code is unchanged.
- Release LTO avoids upstream's legacy/new I2C link conflict. The full original
  device-flash backup is retained locally under ignored `target/`.

## Reproducing verification

```sh
cargo +esp test --workspace
cargo +esp run -p hmi-simulator -- --board living --page home --output artifacts/living-home.png
cargo +esp run -p hmi-simulator -- --board living --page clock --output artifacts/living-clock.png
cargo +esp run -p hmi-simulator -- --board living --page settings --output artifacts/living-settings.png
```

USB-only local controls allow repeatable physical-board checks: `t` clock, `b` BOOT
click, `B` BOOT hold, `k` KEY click, `K` KEY hold, `s` status. They route through the
same application state logic via explicitly framed `@LIVING` lines; they do not prove physical button presses or sound
recognition. `scripts/observe-device.py --say time` plays through the Mac speaker
and tests the real microphone path if the board can hear that output.

No credentials or credential-bearing firmware images are published.

## Verification status

- Host rule, display, persistence, timing and sound tests pass during implementation.
- On-board Wi-Fi/DHCP and network time verified after SSID correction.
- Landscape firmware boots, reports advancing generations and saves/restores a
  Conway world from SD across a reboot.
- **44 host tests passed**, including all existing core tests, landscape corner
  mapping, Life/HighLife/Brain patterns, all 16 rule bounds, independent world
  switching, clock/tour timing, corrupted snapshots and interrupted-save recovery,
  synthetic snap detection, silence/tone rejection and learning timeout.
- Seven generated sound fixtures passed: two "time" voices/rates accepted, five
  unrelated words rejected. These are a small regression set, not an accuracy study.
- Physical-board USB controls cycled all 16 worlds, wrote each to SD and returned
  to Conway at its previous generation. A subsequent restart loaded all 16 worlds.
- Found and fixed a startup memory-order issue: initialize the radio before loading
  all world grids so the latter do not consume its internal-RAM allocations.
- Physical microphone detected "time" played from the Mac's built-in speaker;
  clock appeared at generation 1634 and disappeared at 1654 five seconds later.
  A separate acoustic "hello" test did not trigger. The final cutoff was tightened
  after a synthetic "hello" false positive was caught by the regression script.
- Finger snaps have a synthetic broadband-transient test; a human finger snap and
  the user's own "time" pronunciation have not been directly verified. Use Teach
  Time if the supplied fingerprints do not match reliably. Sound matching remains
  approximate and voice/room dependent.
- Original firmware backup remains local and ignored. No firmware binaries are
  published because they embed local Wi-Fi credentials.
- Larger-grid RLCD hardware check: all sixteen 100 × 62 saved worlds were read,
  expanded to 200 × 124 and saved on the FAT SD card, with no write errors after
  using 8.3-compatible migration archive filenames. A reboot restored the
  increased Conway generation from the new snapshot; a subsequent 30-second
  checkpoint saved all sixteen worlds. The main loop stayed around 91–94 Hz
  and the panel flush took about 4 ms. The stored generation pace setting was
  preserved. Network time and the preferred Wi-Fi network remained available.
