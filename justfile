
# Flash and run the app on actual hardware logging to RTT (USB)
run-rtt:
    cargo run --release --no-default-features --features log-rtt

# Build firmaare for logging to the RTT (USB) 
build-rtt:
    cargo build --no-default-features --features log-rtt

# Lint and format    
precommit:
    cargo clippy
    cargo fmt
    cargo sort