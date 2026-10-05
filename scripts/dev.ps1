$ErrorActionPreference = "Stop"

if (Get-Command nexus -ErrorAction SilentlyContinue) {
    nexus demo
} else {
    cargo run --bin nexus -- demo
}
