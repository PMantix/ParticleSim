# PowerShell script for fresh ParticleSim build with dependency management
# Usage: .\fresh-build.ps1

Write-Host "🚀 Fresh ParticleSim Build with Dependency Management" -ForegroundColor Green

# Set environment variable to trigger fresh dependency build
$env:FRESH_DEPS = "1"

Write-Host "📦 Cleaning previous build..." -ForegroundColor Yellow
cargo clean

Write-Host "🔧 Building with fresh dependencies..." -ForegroundColor Yellow
cargo build --bin particle_sim

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Build successful! Starting ParticleSim..." -ForegroundColor Green
    Write-Host ""
    cargo run --bin particle_sim
} else {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}

# Clean up environment variable
Remove-Item Env:FRESH_DEPS -ErrorAction SilentlyContinue