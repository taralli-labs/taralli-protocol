#!/bin/bash

# Use the CRATE_DIR environment variable set by the build script
CRATE_DIR="${CRATE_DIR:?CRATE_DIR must be set}"

# Navigate to the root of the workspace
WORKSPACE_ROOT=$(realpath "$CRATE_DIR/../..")

# Navigate to the contracts directory
cd "$WORKSPACE_ROOT/contracts" || { echo "Error: Cannot navigate to contracts directory."; exit 1; }

# Check if UniversalBombetta.sol has changed since the last commit
if git diff --quiet HEAD -- src/UniversalBombetta.sol; then
    echo "UniversalBombetta.sol has not changed. Skipping ABI update."
    exit 0
fi

# Clean and compile the contracts
forge clean
echo "Compiling UniversalBombetta.sol..."
forge build --force || { echo "Error: Forge build failed."; exit 1; }

# Navigate back to the original crate directory
cd "$CRATE_DIR" || { echo "Error: Cannot navigate back to crate directory."; exit 1; }

# Define the paths for the ABI file and destination
SRC_PATH="$WORKSPACE_ROOT/contracts/out/UniversalBombetta.sol/UniversalBombetta.json"
DEST_PATH="UniversalBombetta.json"

# Check if the ABI file exists and copy it
if [ -f "$SRC_PATH" ]; then
    cp "$SRC_PATH" "$DEST_PATH"
    echo "UniversalBombetta.json updated successfully!"
else
    echo "Error: ABI file does not exist at $SRC_PATH."
    exit 1
fi