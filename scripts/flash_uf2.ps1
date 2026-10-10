$ErrorActionPreference = "Stop"

& "$PSScriptRoot\build_uf2.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& "$PSScriptRoot\copy_uf2.ps1" "kotsubu-rmk.uf2"
exit $LASTEXITCODE
