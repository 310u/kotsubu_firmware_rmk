#!/usr/bin/env bash
set -euo pipefail

export PATH="${HOME}/.cargo/bin:${PATH}"

cargo objcopy --release --bin kotsubu-rmk -- -O ihex kotsubu-rmk.hex
cargo hex-to-uf2 --input-path kotsubu-rmk.hex --output-path kotsubu-rmk.uf2 --family nrf52840

echo "Generated kotsubu-rmk.hex"
echo "Generated kotsubu-rmk.uf2"
