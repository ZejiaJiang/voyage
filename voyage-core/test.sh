#!/bin/bash
# Test and development script for voyage-core on macOS/Linux
# For full iOS/macOS builds, use build-apple.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}=== Voyage Core Test Script ===${NC}"
echo ""

# Helper functions
success() { echo -e "${GREEN}$1${NC}"; }
warning() { echo -e "${YELLOW}$1${NC}"; }
error() { echo -e "${RED}$1${NC}"; }
info() { echo -e "${CYAN}$1${NC}"; }

# Check requirements
check_requirements() {
    echo "Checking requirements..."
    
    if ! command -v rustup &> /dev/null; then
        error "Error: rustup is not installed"
        echo "Install from: https://rustup.rs"
        exit 1
    fi
    
    if ! command -v cargo &> /dev/null; then
        error "Error: cargo is not installed"
        exit 1
    fi
    
    # Show Rust version
    local rust_version=$(rustc --version)
    success "✓ Rust: $rust_version"
    echo ""
}

# Build for native target
build_native() {
    info "=== Building for native target ==="
    
    cargo build --release
    
    if [ $? -eq 0 ]; then
        success "Build successful"
    else
        error "Build failed"
        exit 1
    fi
    echo ""
}

# Build debug
build_debug() {
    info "=== Building (debug) ==="
    
    cargo build
    
    if [ $? -eq 0 ]; then
        success "Debug build successful"
    else
        error "Build failed"
        exit 1
    fi
    echo ""
}

# Run tests
run_tests() {
    info "=== Running all tests ==="
    
    cargo test
    
    if [ $? -eq 0 ]; then
        success "All tests passed"
    else
        error "Some tests failed"
        exit 1
    fi
    echo ""
}

# Run tests with output
run_tests_verbose() {
    info "=== Running tests (verbose) ==="
    
    cargo test -- --nocapture
    
    echo ""
}

# Run unit tests only
run_unit_tests() {
    info "=== Running unit tests ==="
    
    cargo test --lib
    
    if [ $? -eq 0 ]; then
        success "Unit tests passed"
    else
        error "Unit tests failed"
        exit 1
    fi
    echo ""
}

# Run integration tests only
run_integration_tests() {
    info "=== Running integration tests ==="
    
    cargo test --test integration_test
    
    if [ $? -eq 0 ]; then
        success "Integration tests passed"
    else
        error "Integration tests failed"
        exit 1
    fi
    echo ""
}

# Run demo
run_demo() {
    info "=== Running demo ==="
    
    cargo run --bin demo
    
    echo ""
}

# Run clippy (linter)
run_clippy() {
    info "=== Running Clippy ==="
    
    if command -v cargo-clippy &> /dev/null || cargo clippy --version &> /dev/null; then
        cargo clippy -- -D warnings
        if [ $? -eq 0 ]; then
            success "Clippy passed"
        else
            warning "Clippy found issues"
        fi
    else
        warning "Clippy not installed. Install with: rustup component add clippy"
    fi
    echo ""
}

# Run formatter check
run_fmt_check() {
    info "=== Checking formatting ==="
    
    cargo fmt -- --check
    
    if [ $? -eq 0 ]; then
        success "Formatting OK"
    else
        warning "Formatting issues found. Run 'cargo fmt' to fix."
    fi
    echo ""
}

# Format code
run_fmt() {
    info "=== Formatting code ==="
    
    cargo fmt
    
    success "Code formatted"
    echo ""
}

# Show available targets
show_targets() {
    info "=== Installed Rust Targets ==="
    
    rustup target list --installed
    
    echo ""
    echo "To add iOS/macOS targets, run:"
    echo "  ./build-apple.sh targets"
    echo ""
}

# Print build summary
show_summary() {
    info "=== Build Summary ==="
    echo ""
    
    # Check for various build artifacts
    local artifacts=(
        "target/release/libvoyage_core.dylib:Release dylib"
        "target/release/libvoyage_core.a:Release static lib"
        "target/release/demo:Release demo"
        "target/debug/libvoyage_core.dylib:Debug dylib"
        "target/debug/libvoyage_core.a:Debug static lib"
        "target/debug/demo:Debug demo"
    )
    
    echo "Native Outputs:"
    for entry in "${artifacts[@]}"; do
        local path="${entry%%:*}"
        local desc="${entry##*:}"
        if [ -f "$path" ]; then
            local size=$(ls -lh "$path" | awk '{print $5}')
            echo -e "  ${GREEN}✓${NC} $desc: $size"
        fi
    done
    
    # Check for Apple builds
    echo ""
    echo "Apple Builds:"
    local apple_libs=(
        "target/aarch64-apple-ios/release/libvoyage_core.a:iOS Device"
        "target/aarch64-apple-ios-sim/release/libvoyage_core.a:iOS Sim (arm64)"
        "target/universal-ios-sim/release/libvoyage_core.a:iOS Sim (Universal)"
        "target/aarch64-apple-darwin/release/libvoyage_core.a:macOS (arm64)"
        "target/x86_64-apple-darwin/release/libvoyage_core.a:macOS (x86_64)"
        "target/universal-macos/release/libvoyage_core.a:macOS (Universal)"
    )
    
    local has_apple=false
    for entry in "${apple_libs[@]}"; do
        local path="${entry%%:*}"
        local desc="${entry##*:}"
        if [ -f "$path" ]; then
            local size=$(ls -lh "$path" | awk '{print $5}')
            echo -e "  ${GREEN}✓${NC} $desc: $size"
            has_apple=true
        fi
    done
    
    if [ "$has_apple" = false ]; then
        echo -e "  ${YELLOW}No Apple builds found. Run ./build-apple.sh${NC}"
    fi
    
    # Check for generated bindings
    echo ""
    echo "Swift Bindings:"
    if [ -d "generated" ] && [ "$(ls -A generated/*.swift 2>/dev/null)" ]; then
        for f in generated/*.swift; do
            echo -e "  ${GREEN}✓${NC} $(basename $f)"
        done
    else
        echo -e "  ${YELLOW}Not generated. Run ./build-apple.sh bindings${NC}"
    fi
    
    echo ""
}

# Run all checks (CI-style)
run_ci() {
    info "=== Running CI Checks ==="
    echo ""
    
    check_requirements
    run_fmt_check
    run_clippy
    build_native
    run_tests
    show_summary
    
    success "=== All CI checks passed ==="
}

# Clean build artifacts
clean() {
    info "Cleaning build artifacts..."
    
    cargo clean
    
    if [ -d "generated" ]; then
        rm -rf generated
        echo "Removed generated/"
    fi
    
    success "Cleaned"
}

# Watch for changes and run tests
watch_tests() {
    info "=== Watching for changes ==="
    
    if command -v cargo-watch &> /dev/null; then
        cargo watch -x test
    else
        warning "cargo-watch not installed."
        echo "Install with: cargo install cargo-watch"
        echo "Then run: cargo watch -x test"
    fi
}

# Show help
show_help() {
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  check      - Check build requirements"
    echo "  build      - Build release for native target"
    echo "  debug      - Build debug for native target"
    echo "  test       - Run all tests"
    echo "  test-v     - Run tests with output (verbose)"
    echo "  unit       - Run unit tests only"
    echo "  integration- Run integration tests only"
    echo "  demo       - Run the demo binary"
    echo "  clippy     - Run Clippy linter"
    echo "  fmt        - Format code"
    echo "  fmt-check  - Check formatting"
    echo "  targets    - Show installed Rust targets"
    echo "  summary    - Show build summary"
    echo "  ci         - Run all CI checks"
    echo "  clean      - Clean build artifacts"
    echo "  watch      - Watch and run tests on changes"
    echo "  all        - Build and test (default)"
    echo "  help       - Show this help"
    echo ""
    echo "For iOS/macOS builds, use: ./build-apple.sh"
}

# Main execution
main() {
    local command=${1:-all}
    
    case $command in
        check)
            check_requirements
            ;;
        build)
            check_requirements
            build_native
            ;;
        debug)
            check_requirements
            build_debug
            ;;
        test)
            check_requirements
            run_tests
            ;;
        test-v|test-verbose)
            check_requirements
            run_tests_verbose
            ;;
        unit)
            check_requirements
            run_unit_tests
            ;;
        integration|int)
            check_requirements
            run_integration_tests
            ;;
        demo)
            check_requirements
            run_demo
            ;;
        clippy|lint)
            run_clippy
            ;;
        fmt|format)
            run_fmt
            ;;
        fmt-check)
            run_fmt_check
            ;;
        targets)
            show_targets
            ;;
        summary)
            show_summary
            ;;
        ci)
            run_ci
            ;;
        clean)
            clean
            ;;
        watch)
            watch_tests
            ;;
        all)
            check_requirements
            build_native
            run_tests
            show_summary
            ;;
        help|-h|--help)
            show_help
            ;;
        *)
            error "Unknown command: $command"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# Run main with all arguments
main "$@"
