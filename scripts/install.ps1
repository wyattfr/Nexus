$ErrorActionPreference = "Stop"

Write-Host "Installing NEXUS CLI..."

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo was not found. Install Rust from https://rustup.rs/ and reopen PowerShell."
}

cargo install --path .\crates\nexus-cli --force

$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME ".cargo" }
$cargoBin = Join-Path $cargoHome "bin"
$nexusExe = Join-Path $cargoBin "nexus.exe"

if (-not (Test-Path $nexusExe)) {
    throw "Installation completed but nexus.exe was not found at $nexusExe"
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @()
if ($userPath) {
    $pathEntries = $userPath -split ";" | Where-Object { $_ }
}
if ($pathEntries -notcontains $cargoBin) {
    $newUserPath = (($pathEntries + $cargoBin) -join ";")
    [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
    Write-Host "Added $cargoBin to the user PATH."
} else {
    Write-Host "$cargoBin is already on the user PATH."
}

if (($env:Path -split ";") -notcontains $cargoBin) {
    $env:Path = "$cargoBin;$env:Path"
    Write-Host "Updated the current PowerShell PATH."
}

Write-Host ""
Write-Host "Installed: $nexusExe"
Write-Host "NEXUS version:"
& $nexusExe --version
Write-Host "NEXUS help:"
& $nexusExe --help
Write-Host ""
Write-Host "If another terminal does not find nexus, close and reopen it."
