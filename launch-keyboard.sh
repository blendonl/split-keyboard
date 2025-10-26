#!/bin/bash
# Launch script for split keyboard

BINARY="./target/release/split-keyboard"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Binary not found. Building..."
    cargo build --release
fi

# Kill any existing instances
pkill -f "split-keyboard"

# Start both keyboards
echo "Starting split keyboard..."
$BINARY left &
LEFT_PID=$!

$BINARY right &
RIGHT_PID=$!

echo "Split keyboard started"
echo "Left keyboard PID: $LEFT_PID"
echo "Right keyboard PID: $RIGHT_PID"
echo ""
echo "To stop: pkill split-keyboard"
