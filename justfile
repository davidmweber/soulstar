# Run with debugging information
run:
    cargo run

# Build
build:
    cargo build

# Flash and run the app in fully optimesed release mode
run-release:
    cargo run --release

# Build for release mode
build-release:
    cargo build --release

# Lint and format    
precommit:
    cargo clippy
    cargo fmt
    cargo sort