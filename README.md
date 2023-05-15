# Drippy Metro-Tap

## Setting up the Development Environment

### ESP IoT Development Framework (ESP-IDF)

```bash
# Install build dependencies for the ESP IoT Development Framework (ESP-IDF).
sudo apt update
sudo apt install git-all python3 -y
cargo install ldproxy
```

### Xtensa Toolchain

```bash
# Enable installing pre-built binaries without building them from scratch with `cargo install`.
cargo install binstall

# Set up Espressif's custom toolchain (forked from LLVM and `rustc`) for Xtensa architecture support.
cargo binstall espup
espup install

# Load toolchain-related environment variables.
source $HOME/export-esp.sh
```

### Extra Tooling for Convenient Code Flashing and Uploading

```bash
cargo binstall espflash
cargo binstall cargo-espflash
```

### Running the Project

```bash
# Builds the project in release mode, flashes the ESP32 chip, and hooks into the serial monitor.
cargo run --release
```
