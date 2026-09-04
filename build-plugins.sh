#!/usr/bin/bash

echo "🔔 Build fx-plugin wasm..."
cargo b -p fx-plugin -r --target wasm32-wasip1

echo "🔔 Build xx-plugin wasm..."
cargo b -p xx-plugin -r --target wasm32-wasip1

echo "🔔 Copy WASM files to 'server' plugins folder..."
mkdir server/plugins || true
cp -f target/wasm32-wasip1/release/*.wasm server/plugins/
