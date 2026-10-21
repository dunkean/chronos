# Chronos demo: build, open the browser, serve the workshop on localhost.
# Usage: .\demo.ps1 [-Port 8785] [-NoBrowser]
param([int]$Port = 8785, [switch]$NoBrowser)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

cargo build --release -p chronos-app
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if (-not $NoBrowser) { Start-Process "http://127.0.0.1:$Port/" }
& "$PSScriptRoot\target\release\chronos-app.exe" serve --port $Port
exit $LASTEXITCODE
