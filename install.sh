#!/bin/sh

# An install script for Sarch32
# Compiles and installs in /usr/local/bin

cargo build --release

echo "Trying to install in /usr/local/bin"
echo "Requesting permission..."
sudo cp ./target/release/sarch_asm /usr/local/bin/

