param([switch]$SkipChecks)
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$ProjectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $ProjectRoot
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Install Rust with rustup (https://rustup.rs/) and Visual Studio C++ Build Tools first."
    }
    if (-not $SkipChecks) {
        cargo fmt --check
        if ($LASTEXITCODE -ne 0) { throw "Formatting failed" }
        cargo test --locked
        if ($LASTEXITCODE -ne 0) { throw "Tests failed" }
    }
    cargo build --release --locked --bins --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }
    $Portable = Join-Path $ProjectRoot "dist/Telysta-Windows-x64"
    New-Item -ItemType Directory -Force -Path $Portable | Out-Null
    Copy-Item "target/x86_64-pc-windows-msvc/release/telysta.exe" "$Portable/Telysta.exe" -Force
    Copy-Item "target/x86_64-pc-windows-msvc/release/telysta-cli.exe" "$Portable/telysta-cli.exe" -Force
    Copy-Item "README.md", "LICENSE", "THIRD-PARTY-NOTICES.md" $Portable -Force
    Copy-Item "assets/fonts/OFL.txt" "$Portable/FONT-LICENSE.txt" -Force
    Copy-Item "assets/licenses/UNRAR.txt" "$Portable/UNRAR-LICENSE.txt" -Force
    New-Item -ItemType Directory -Force -Path "$Portable/docs" | Out-Null
    Copy-Item "docs/preview.png" "$Portable/docs/preview.png" -Force
    Copy-Item "docs/使用说明.txt" "$Portable/使用说明.txt" -Force
    Compress-Archive -Path $Portable -DestinationPath "dist/Telysta-Windows-x64.zip" -Force
    Write-Host "Ready: $Portable/Telysta.exe" -ForegroundColor Green
    Write-Host "Portable package: dist/Telysta-Windows-x64.zip" -ForegroundColor Green
} finally {
    Pop-Location
}
