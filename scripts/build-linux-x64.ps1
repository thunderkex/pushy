# Build static Linux x86_64 binary locally on Windows using Zig cross-toolchain
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir

$env:Path = "$ScriptDir;" + [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
$env:CC_x86_64_unknown_linux_musl = "$ScriptDir\x86_64-linux-musl-gcc.cmd"
$env:AR_x86_64_unknown_linux_musl = "$ScriptDir\x86_64-linux-musl-ar.cmd"

Set-Location $ProjectDir
cargo build --target x86_64-unknown-linux-musl --release

Write-Host "`nLinux binary generated at:" -ForegroundColor Green
Write-Host "$ProjectDir\target\x86_64-unknown-linux-musl\release\pushy" -ForegroundColor Cyan
