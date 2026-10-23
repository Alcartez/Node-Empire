# Node Empire — Setup

Node Empire is a Rust + Bevy application. Rust **1.98** is pinned in
`rust-toolchain.toml`; `rustup` will install the right toolchain automatically.

## Windows

1. **C++ build tools.** Install the
   [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   and select **Desktop development with C++** (MSVC compiler + Windows SDK).
2. **Rust.** Download [`rustup-init.exe`](https://win.rustup.rs/), run it, and
   accept the default toolchain (`1`). Restart your terminal.
3. **Verify:**
   ```powershell
   rustc --version
   cargo --version
   ```

## Linux

Install the graphics, audio, and input libraries Bevy needs, then `rustup`.

**Ubuntu / Debian / Mint**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libudev-dev libasound2-dev \
                    libwayland-dev libxkbcommon-dev libx11-dev
```

**Fedora / RHEL**
```bash
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y pkg-config systemd-devel alsa-lib-devel \
                    wayland-devel libxkbcommon-devel libX11-devel
```

**Arch / Manjaro**
```bash
sudo pacman -Syu --needed base-devel pkgconf alsa-lib systemd-libs \
                       wayland libxkbcommon libx11
```

Then:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

## macOS

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

## Build and run

```bash
cargo test                 # headless simulation; no window required
cargo run --release        # play
```

Use `--release` for playing. The dev profile is partly optimized because Bevy is
slow unoptimized, but release is still the intended experience.

Add the lint components if `rustup` did not:
```bash
rustup component add clippy rustfmt
```

`Cargo.lock` is committed, so the dependency set is reproducible.

## Saves and logs

- Saves: `saves/` in the working directory (`autosave.json`, `save_slot_1.json`,
  …).
- Autosave every 600 ticks; `0` quick-saves to slot 1.
- Saves are JSON and survive schema additions; newer-version saves are refused.

If the game fails to start, first confirm `cargo test` passes. A missing system
library shows up as a link error before any game code runs.
