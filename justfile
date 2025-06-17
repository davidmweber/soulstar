
# Flash and run the app on actual hardware logging to RTT (USB)
flash-run-rtt:
    cargo run --no-default-features --features log-rtt

# Build firmaare for logging to the RTT (USB) 
build-hardware-rtt:
    cargo build --no-default-features --features log-rtt

# Lint and format    
precommit:
    cargo clippy
    cargo fmt