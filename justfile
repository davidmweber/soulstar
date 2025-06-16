

# Flash and run the app on actual hardware
flash-run:
    cargo run --no-default-features --features log-rtt


# Build it for the target hardware. 
# This just ensures that the logging output goes to RTT instead of the UART for probe-rs to consume
build-hardware:
    cargo build --no-default-features --features log-rtt