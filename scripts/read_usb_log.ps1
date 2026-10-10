param(
    [string]$Port,
    [string]$OutFile = "ble_log.txt"
)

if (-not $Port) {
    $pnp = Get-CimInstance Win32_PnPEntity |
        Where-Object { $_.DeviceID -match 'VID_4C4B&PID_4B32' -and $_.Name -match 'COM\d+' } |
        Select-Object -First 1
    if (-not $pnp) {
        Write-Host "CDC serial port not found. Is the keyboard connected via USB?"
        exit 1
    }
    $Port = [regex]::Match($pnp.Name, 'COM\d+').Value
}

Write-Host "Reading $Port -> $OutFile (Ctrl+C to stop)"
$sp = New-Object System.IO.Ports.SerialPort $Port, 115200, None, 8, One
$sp.ReadTimeout = 1000
$sp.Open()
$sw = New-Object System.IO.StreamWriter $OutFile
$sw.AutoFlush = $true
try {
    while ($true) {
        try {
            $line = $sp.ReadLine()
            if ($null -ne $line) {
                Write-Host $line
                $sw.WriteLine($line)
            }
        }
        catch [System.TimeoutException] { }
    }
}
finally {
    $sp.Close()
    $sw.Close()
    Write-Host "Saved to $OutFile"
}
