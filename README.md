# bpew

Play a terminal audio notification. No sound files required.

`bpew` loops a three-note chime with a two-second pause between repetitions.
Press Ctrl+C to stop. Default volume is 35%.

## Install

Requires Rust 1.87 or newer and a working default audio output.
On Linux, install ALSA development headers and pkg-config before building:

```sh
# Debian / Ubuntu
sudo apt install libasound2-dev pkg-config
# Fedora
sudo dnf install alsa-lib-devel pkgconf-pkg-config
# Arch
sudo pacman -S alsa-lib pkgconf
```

```sh
cargo install --path . --locked
```

The executable goes in `~/.cargo/bin`. Playback uses the default system audio
output through Rodio. macOS and Windows do not need the Linux packages above.

## Use

```sh
# Notify after the command finishes, whether it succeeds or fails
wget -c http://example.com/big.zip ; bpew

# Notify only on success, just once
cargo build --release && bpew --preset success --once

# Repeat an arcade tune for 10 seconds
bpew --preset arcade --duration 10

# Quieter, gentle reminder for 2.5 seconds
bpew -p gentle -d 2.5 -v 0.2

# A more urgent notification
bpew -p alarm

bpew --list
bpew --help
```

Presets: `chime`, `success`, `arcade`, `alarm`, `gentle`.

`--duration` is a maximum total playback time in seconds, including loop pauses.
It accepts positive decimals. With `--once`, the melody ends naturally or at the
duration limit, whichever comes first. Without either option, playback continues
until you interrupt it. `--volume` accepts values from 0 to 1.

The command stays in the foreground while playing. It needs a working audio
session, so a headless server or SSH session without audio output will report an
error. `--help` and `--list` work without an audio device.

## Check

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The test checks argument validation, all five melodies, loop silence, and duration
limits without opening an audio device.
