$ErrorActionPreference = "Stop"

cargo objcopy --release --bin kotsubu-rmk -- -O ihex kotsubu-rmk.hex
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo hex-to-uf2 --input-path kotsubu-rmk.hex --output-path kotsubu-rmk.uf2 --family nrf52840
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Generated kotsubu-rmk.hex"
Write-Host "Generated kotsubu-rmk.uf2"
