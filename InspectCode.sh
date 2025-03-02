#!/bin/bash

# Get the directory of the script file
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Define target architectures
TARGETS=("riscv64gc-unknown-none-elf")

# Crates that require all features, eg. drivers uses features for multi platform supports
MULTI_FEATURES_CRATES=("drivers")

# Function to run checks for a given directory and target
run_checks() {
  local dir=$1
  local target=$2
  local all_features=$3

  echo ""
  echo "# ====================================================================="
  echo "# Checking $dir folder for target $target..."
  echo "# ====================================================================="
  cd "$SCRIPT_DIR/$dir" || exit

  if [ "$all_features" = true ]; then
    cargo clippy --all-features --target "$target" -- -D warnings
  else
    cargo clippy --features=no_std --target "$target" -- -D warnings
  fi

  cargo fmt --all -- --check
}

# Iterate through each target and run checks for kernel and crates
for target in "${TARGETS[@]}"; do
  run_checks "kernel" "$target" false
  run_checks "crates" "$target" false
done

for crate in "${MULTI_FEATURES_CRATES[@]}"; do
  for target in "${TARGETS[@]}"; do
    run_checks "crates/$crate" "$target" true
  done
done

echo "Code inspection completed."
