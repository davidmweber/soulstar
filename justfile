# Default log level is "error". It can be "error", "info", "debug" or "trace"
default_log := "error"

# Run with debugging information
run log='{{default_log}}':
    DEFMT_LOG={{log}} cargo run

# Build in debug mode
build:
    cargo build

# Flash and run the app in fully optimesed release mode
run-release log='{{default_log}}':
    DEFMT_LOG={{log}} cargo run --release

# Build for release mode
build-release:
    cargo build --release

# Lint and format    
precommit:
    cargo clippy
    cargo fmt
    cargo sort