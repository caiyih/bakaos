#!/bin/bash

# Get the directory of the script file
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check kernel folder
echo "Checking kernel folder..."
cd "$SCRIPT_DIR/kernel" || exit
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check

# Check crates folder
echo "Checking crates folder..."
cd "$SCRIPT_DIR/crates" || exit
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check

echo "Code inspection completed."
