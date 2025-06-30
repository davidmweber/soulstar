# Default log level is "error". It can be "error", "info", "debug" or "trace"
default_log := "error"

# Run with debugging information
run log=default_log:
    DEFMT_LOG={{log}} cargo run

# Build in debug mode
build:
    cargo build

# Flash and run the app in fully optimesed release mode. Note there will be no logging in release mode.
run-release log=default_log:
    DEFMT_LOG={{log}} cargo run --release

# Build for release mode
build-release:
    cargo build --release --chip ESP32-C6

flash soul:
    SOUL_ID={{soul}} cargo flash --release



# Lint and format    
precommit:
    cargo clippy
    cargo fmt
    cargo sort