# justfile
set dotenv-path := "./contracts/.env"

# Variables
STATE_FILE := "anvil_state.json"
# URL is ethereum holesky (has both permit2 and aligned layer testnet deployed on it)
FORK_URL := env("ETH_HOLESKY_RPC_URL")
RPC_URL := env("ETH_LOCAL_RPC_URL")

# Task to spin up an anvil node
start_anvil:
    #!/bin/bash
    if test -f {{STATE_FILE}}; then
        anvil --load-state {{STATE_FILE}} --fork-url {{FORK_URL}}
    else
        anvil --fork-url {{FORK_URL}} --dump-state {{STATE_FILE}}
    fi

# Commands to deploy contract(s)
mock_deploy_contracts:
    cd contracts/ && forge script Deploy -vvvv

deploy_contracts:
    cd contracts/ && forge script Deploy --broadcast

# Server commands
start_server:
    cargo run --bin server

# Client commands
simple_request:
    cargo run --example simple_request

subscribe_market:
    cargo run --example subscribe_market

# Fixes the formatting of the workspace
fmt-fix:
    cargo +nightly fmt --all

# Check the formatting of the workspace
fmt-check:
    cargo +nightly fmt --all -- --check

# Lint the workspace
lint: fmt-check
    cargo +nightly clippy --workspace --all --all-features --all-targets -- -D warnings

# Update market address...everytime a fresh deploy of an anvil devnet occurs you can copy the deployment addr
# of the bombetta market deployed in the forge script and write it into the server's config.json file to be used in the
# server config.
update_market_address:
    #!/bin/bash

    # Paths to the JSON files
    DEPLOYMENTS_FILE="./contracts/deployments.json"
    CONFIG_FILE="./config.json"

    # Extract the universal_bombetta address from the deployments.json file
    UNIVERSAL_BOMBETTA_ADDRESS=$(jq -r '.universal_bombetta' $DEPLOYMENTS_FILE)

    # Check if the universal_bombetta address was successfully extracted
    if [ -z "$UNIVERSAL_BOMBETTA_ADDRESS" ] || [ "$UNIVERSAL_BOMBETTA_ADDRESS" == "null" ]; then
       echo "Error: Could not find 'universal_bombetta' address in $DEPLOYMENTS_FILE."
      exit 1
    fi

    # Update the market_address field in the config.json file
    jq --arg address "$UNIVERSAL_BOMBETTA_ADDRESS" '.market_address = $address' $CONFIG_FILE > tmp.$$.json && mv tmp.$$.json $CONFIG_FILE

    # Check if the update was successful
    if [ $? -eq 0 ]; then
      echo "Successfully updated market_address in $CONFIG_FILE to $UNIVERSAL_BOMBETTA_ADDRESS."
    else
      echo "Error: Failed to update market_address in $CONFIG_FILE."
      exit 1
    fi