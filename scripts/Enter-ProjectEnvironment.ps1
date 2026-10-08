# Dot-source this script: . .\scripts\Enter-ProjectEnvironment.ps1
$projectRoot = Split-Path -Parent $PSScriptRoot
$env:CARGO_HOME = Join-Path $projectRoot '.cache\cargo'
$env:TEMP = Join-Path $projectRoot '.work\tmp'
$env:TMP = $env:TEMP
New-Item -ItemType Directory -Force -Path $env:CARGO_HOME,$env:TEMP | Out-Null
Set-Location -LiteralPath $projectRoot
Write-Host "Project/cache/temp: $projectRoot"
