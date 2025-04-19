#!/bin/sh

# An install script for Sarch32
# Compiles and installs in /usr/local/bin

INSTALL_DIR=/usr/local/bin

cargo build --release

echo "Trying to install in $INSTALL_DIR"
echo "Requesting permission..."
sudo install ./target/release/sarch_asm $INSTALL_DIR

