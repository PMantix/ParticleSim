@echo off
echo 🚀 Fresh ParticleSim Build with Dependency Management

:: Set environment variable for fresh deps
set FRESH_DEPS=1

echo 📦 Cleaning previous build...
cargo clean

echo 🔧 Building with fresh dependencies...
cargo build --bin particle_sim

if %ERRORLEVEL% NEQ 0 (
    echo ❌ Build failed!
    exit /b %ERRORLEVEL%
)

echo ✅ Build successful! Starting ParticleSim...
echo.
cargo run --bin particle_sim

:: Clean up
set FRESH_DEPS=