# Build script for voyage-core on Windows (for testing/development)
# For actual iOS/macOS builds, use build-apple.sh on a Mac

Write-Host "=== Voyage Core Windows Build Script ===" -ForegroundColor Cyan
Write-Host ""

function Write-Success { param($msg) Write-Host $msg -ForegroundColor Green }
function Write-Warning { param($msg) Write-Host $msg -ForegroundColor Yellow }
function Write-Error { param($msg) Write-Host $msg -ForegroundColor Red }

# Check requirements
function Test-Requirements {
    Write-Host "Checking requirements..."
    
    $hasRustup = Get-Command rustup -ErrorAction SilentlyContinue
    if (-not $hasRustup) {
        Write-Error "Error: rustup is not installed"
        Write-Host "Install from: https://rustup.rs"
        exit 1
    }
    
    $hasCargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $hasCargo) {
        Write-Error "Error: cargo is not installed"
        exit 1
    }
    
    Write-Success "All requirements met"
    Write-Host ""
}

# Build for Windows (native)
function Build-Native {
    Write-Host "=== Building for Windows (native) ===" -ForegroundColor Yellow
    
    cargo build --release
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Build successful"
    } else {
        Write-Error "Build failed"
        exit 1
    }
    Write-Host ""
}

# Run tests
function Run-Tests {
    Write-Host "=== Running tests ===" -ForegroundColor Yellow
    
    cargo test
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "All tests passed"
    } else {
        Write-Error "Some tests failed"
        exit 1
    }
    Write-Host ""
}

# Run demo
function Run-Demo {
    Write-Host "=== Running demo ===" -ForegroundColor Yellow
    
    cargo run --bin demo
    
    Write-Host ""
}

# Check what can be built
function Show-Targets {
    Write-Host "=== Available Rust Targets ===" -ForegroundColor Yellow
    
    $targets = rustup target list --installed
    Write-Host "Installed targets:"
    foreach ($target in $targets) {
        Write-Host "  - $target"
    }
    
    Write-Host ""
    Write-Host "To build for iOS/macOS, you need a Mac. Use build-apple.sh on macOS."
    Write-Host ""
}

# Print build summary
function Show-Summary {
    Write-Host "=== Build Summary ===" -ForegroundColor Cyan
    Write-Host ""
    
    $libPath = "target\release\voyage_core.dll"
    $staticPath = "target\release\libvoyage_core.a"
    $demoPath = "target\release\demo.exe"
    
    Write-Host "Windows Outputs:"
    
    if (Test-Path $libPath) {
        $size = (Get-Item $libPath).Length / 1KB
        Write-Host "  [OK] voyage_core.dll: $([math]::Round($size, 1)) KB" -ForegroundColor Green
    }
    
    if (Test-Path $staticPath) {
        $size = (Get-Item $staticPath).Length / 1KB
        Write-Host "  [OK] libvoyage_core.a: $([math]::Round($size, 1)) KB" -ForegroundColor Green
    }
    
    if (Test-Path $demoPath) {
        $size = (Get-Item $demoPath).Length / 1KB
        Write-Host "  [OK] demo.exe: $([math]::Round($size, 1)) KB" -ForegroundColor Green
    }
    
    Write-Host ""
    Write-Host "Note: For iOS/macOS builds, transfer this project to a Mac and run:" -ForegroundColor Yellow
    Write-Host "  chmod +x build-apple.sh"
    Write-Host "  ./build-apple.sh"
    Write-Host ""
}

# Clean build artifacts
function Clean-Build {
    Write-Host "Cleaning build artifacts..." -ForegroundColor Yellow
    
    cargo clean
    
    if (Test-Path "generated") {
        Remove-Item -Recurse -Force "generated"
    }
    
    Write-Success "Cleaned"
}

# Main execution
$command = $args[0]
if (-not $command) { $command = "all" }

switch ($command) {
    "check" {
        Test-Requirements
    }
    "build" {
        Test-Requirements
        Build-Native
    }
    "test" {
        Test-Requirements
        Run-Tests
    }
    "demo" {
        Test-Requirements
        Run-Demo
    }
    "targets" {
        Show-Targets
    }
    "summary" {
        Show-Summary
    }
    "clean" {
        Clean-Build
    }
    "all" {
        Test-Requirements
        Build-Native
        Run-Tests
        Show-Summary
    }
    default {
        Write-Host "Usage: .\build-windows.ps1 [command]"
        Write-Host ""
        Write-Host "Commands:"
        Write-Host "  check   - Check build requirements"
        Write-Host "  build   - Build for Windows"
        Write-Host "  test    - Run tests"
        Write-Host "  demo    - Run the demo"
        Write-Host "  targets - Show available targets"
        Write-Host "  summary - Show build summary"
        Write-Host "  clean   - Clean build artifacts"
        Write-Host "  all     - Build and test (default)"
        Write-Host ""
        Write-Host "For iOS/macOS builds, use build-apple.sh on a Mac"
    }
}
