#!/bin/bash

# This script is used to build and run the project in a 'test' folder, so that
# the project can be tested without cluttering the main folder.

# Build the project
cargo build --release

# cd into the test folder
mkdir -p test
cd test

# Copy the executable into the test folder
cp ../target/release/snap_rs .

# Run the executable
./snap_rs