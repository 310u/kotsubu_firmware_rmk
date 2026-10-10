param([string]$Uf2File = "kotsubu-rmk.uf2")

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Uf2File)) {
    Write-Error "UF2 file not found: $Uf2File"
    exit 1
}

$targets = @(
    Get-PSDrive -PSProvider FileSystem |
        ForEach-Object { $_.Root } |
        Where-Object { $_ -and (Test-Path (Join-Path $_ "INFO_UF2.TXT")) }
)

if ($targets.Count -eq 0) {
    Write-Error @"
UF2 bootloader drive was not found.
Put the XIAO BLE into bootloader mode first, then re-run this command.
The mounted drive should contain INFO_UF2.TXT.
"@
    exit 1
}

if ($targets.Count -gt 1) {
    Write-Error ("Multiple UF2 drives were found: " + ($targets -join ", ") + ". Please keep only one and retry.")
    exit 1
}

$dest = Join-Path $targets[0] (Split-Path $Uf2File -Leaf)
Write-Host "Copying $Uf2File -> $dest"
Copy-Item $Uf2File $dest
Write-Host "UF2 copy finished."
