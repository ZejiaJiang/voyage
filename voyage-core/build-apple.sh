#!/bin/bash
# Build script for voyage-core on macOS/iOS
# Run this script on a Mac with Xcode and Rust installed

set -e

echo "=== Voyage Core Apple Build Script ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check for required tools
check_requirements() {
    echo "Checking requirements..."
    
    if ! command -v rustup &> /dev/null; then
        echo -e "${RED}Error: rustup is not installed${NC}"
        echo "Install from: https://rustup.rs"
        exit 1
    fi
    
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: cargo is not installed${NC}"
        exit 1
    fi
    
    if ! command -v xcrun &> /dev/null; then
        echo -e "${RED}Error: Xcode command line tools not found${NC}"
        echo "Install with: xcode-select --install"
        exit 1
    fi
    
    echo -e "${GREEN}All requirements met${NC}"
    echo ""
}

# Install required Rust targets
install_targets() {
    echo "Installing Rust targets..."
    
    # iOS targets
    rustup target add aarch64-apple-ios          # iOS device (arm64)
    rustup target add aarch64-apple-ios-sim      # iOS Simulator (arm64, Apple Silicon)
    rustup target add x86_64-apple-ios           # iOS Simulator (x86_64, Intel)
    
    # macOS targets
    rustup target add aarch64-apple-darwin       # macOS (Apple Silicon)
    rustup target add x86_64-apple-darwin        # macOS (Intel)
    
    echo -e "${GREEN}Targets installed${NC}"
    echo ""
}

# Build for a specific target
build_target() {
    local target=$1
    local profile=${2:-release}
    
    echo -e "${YELLOW}Building for ${target}...${NC}"
    cargo build --target "$target" --"$profile"
    echo -e "${GREEN}Built ${target}${NC}"
}

# Build iOS targets
build_ios() {
    echo "=== Building iOS targets ==="
    
    build_target "aarch64-apple-ios" "release"
    build_target "aarch64-apple-ios-sim" "release"
    build_target "x86_64-apple-ios" "release"
    
    echo ""
}

# Build macOS targets
build_macos() {
    echo "=== Building macOS targets ==="
    
    build_target "aarch64-apple-darwin" "release"
    build_target "x86_64-apple-darwin" "release"
    
    echo ""
}

# Create universal (fat) library for iOS Simulator
create_ios_sim_universal() {
    echo "=== Creating iOS Simulator universal library ==="
    
    local arm64_lib="target/aarch64-apple-ios-sim/release/libvoyage_core.a"
    local x86_lib="target/x86_64-apple-ios/release/libvoyage_core.a"
    local output_dir="target/universal-ios-sim/release"
    local output_lib="$output_dir/libvoyage_core.a"
    
    mkdir -p "$output_dir"
    
    if [ -f "$arm64_lib" ] && [ -f "$x86_lib" ]; then
        lipo -create "$arm64_lib" "$x86_lib" -output "$output_lib"
        echo -e "${GREEN}Created universal iOS Simulator library: $output_lib${NC}"
    else
        echo -e "${YELLOW}Warning: Could not create universal library, missing one or more architectures${NC}"
    fi
    
    echo ""
}

# Create universal (fat) library for macOS
create_macos_universal() {
    echo "=== Creating macOS universal library ==="
    
    local arm64_lib="target/aarch64-apple-darwin/release/libvoyage_core.a"
    local x86_lib="target/x86_64-apple-darwin/release/libvoyage_core.a"
    local output_dir="target/universal-macos/release"
    local output_lib="$output_dir/libvoyage_core.a"
    
    mkdir -p "$output_dir"
    
    if [ -f "$arm64_lib" ] && [ -f "$x86_lib" ]; then
        lipo -create "$arm64_lib" "$x86_lib" -output "$output_lib"
        echo -e "${GREEN}Created universal macOS library: $output_lib${NC}"
    else
        echo -e "${YELLOW}Warning: Could not create universal library, missing one or more architectures${NC}"
    fi
    
    echo ""
}

# Generate Swift bindings using UniFFI
generate_bindings() {
    echo "=== Generating Swift bindings ==="
    
    local bindings_dir="generated"
    mkdir -p "$bindings_dir"
    
    # Generate bindings using uniffi-bindgen
    if cargo run --bin uniffi-bindgen generate \
        --library target/release/libvoyage_core.dylib \
        --language swift \
        --out-dir "$bindings_dir" 2>/dev/null; then
        echo -e "${GREEN}Swift bindings generated in $bindings_dir${NC}"
    else
        # Fallback: try with UDL file directly
        echo "Trying alternate binding generation method..."
        if cargo run --features=uniffi/cli --bin uniffi-bindgen -- \
            generate src/voyage_core.udl \
            --language swift \
            --out-dir "$bindings_dir" 2>/dev/null; then
            echo -e "${GREEN}Swift bindings generated in $bindings_dir${NC}"
        else
            echo -e "${YELLOW}Note: Could not auto-generate bindings. You may need to run:${NC}"
            echo "  cargo install uniffi_bindgen"
            echo "  uniffi-bindgen generate src/voyage_core.udl --language swift --out-dir $bindings_dir"
        fi
    fi
    
    echo ""
}

# Create XCFramework for distribution
create_xcframework() {
    echo "=== Creating XCFramework ==="
    
    local output_dir="VoyageCore.xcframework"
    local header_dir="generated"
    
    # Check if we have the required files
    if [ ! -f "target/aarch64-apple-ios/release/libvoyage_core.a" ]; then
        echo -e "${RED}Error: iOS device library not found. Run build first.${NC}"
        return 1
    fi
    
    # Remove existing xcframework
    rm -rf "$output_dir"
    
    # Create module.modulemap if it doesn't exist
    mkdir -p "$header_dir"
    cat > "$header_dir/module.modulemap" << 'EOF'
framework module VoyageCore {
    umbrella header "voyage_coreFFI.h"
    export *
    module * { export * }
}
EOF
    
    # Create XCFramework
    xcodebuild -create-xcframework \
        -library target/aarch64-apple-ios/release/libvoyage_core.a \
        -headers "$header_dir" \
        -library target/universal-ios-sim/release/libvoyage_core.a \
        -headers "$header_dir" \
        -library target/universal-macos/release/libvoyage_core.a \
        -headers "$header_dir" \
        -output "$output_dir"
    
    echo -e "${GREEN}Created XCFramework: $output_dir${NC}"
    echo ""
}

# Copy outputs to Xcode projects
copy_to_projects() {
    echo "=== Copying outputs to Xcode projects ==="
    
    local ios_project="../Voyage"
    local macos_project="../VoyageMac"
    local bindings_dir="generated"
    
    # Copy to iOS project
    if [ -d "$ios_project" ]; then
        echo "Copying to iOS project..."
        mkdir -p "$ios_project/VoyageCore"
        
        # Copy static library for iOS device
        cp -f target/aarch64-apple-ios/release/libvoyage_core.a \
            "$ios_project/VoyageCore/libvoyage_core_ios.a" 2>/dev/null || true
        
        # Copy Swift bindings
        cp -f "$bindings_dir"/*.swift "$ios_project/VoyageCore/" 2>/dev/null || true
        cp -f "$bindings_dir"/*.h "$ios_project/VoyageCore/" 2>/dev/null || true
        
        echo -e "${GREEN}Copied to iOS project${NC}"
    else
        echo -e "${YELLOW}iOS project not found at $ios_project${NC}"
    fi
    
    # Copy to macOS project
    if [ -d "$macos_project" ]; then
        echo "Copying to macOS project..."
        mkdir -p "$macos_project/VoyageCore"
        
        # Copy universal macOS library
        cp -f target/universal-macos/release/libvoyage_core.a \
            "$macos_project/VoyageCore/libvoyage_core_macos.a" 2>/dev/null || true
        
        # Copy Swift bindings
        cp -f "$bindings_dir"/*.swift "$macos_project/VoyageCore/" 2>/dev/null || true
        cp -f "$bindings_dir"/*.h "$macos_project/VoyageCore/" 2>/dev/null || true
        
        echo -e "${GREEN}Copied to macOS project${NC}"
    else
        echo -e "${YELLOW}macOS project not found at $macos_project${NC}"
    fi
    
    echo ""
}

# Print summary of built artifacts
print_summary() {
    echo "=== Build Summary ==="
    echo ""
    echo "Static Libraries:"
    
    local libs=(
        "target/aarch64-apple-ios/release/libvoyage_core.a:iOS Device (arm64)"
        "target/aarch64-apple-ios-sim/release/libvoyage_core.a:iOS Simulator (arm64)"
        "target/x86_64-apple-ios/release/libvoyage_core.a:iOS Simulator (x86_64)"
        "target/universal-ios-sim/release/libvoyage_core.a:iOS Simulator (Universal)"
        "target/aarch64-apple-darwin/release/libvoyage_core.a:macOS (arm64)"
        "target/x86_64-apple-darwin/release/libvoyage_core.a:macOS (x86_64)"
        "target/universal-macos/release/libvoyage_core.a:macOS (Universal)"
    )
    
    for entry in "${libs[@]}"; do
        local path="${entry%%:*}"
        local desc="${entry##*:}"
        if [ -f "$path" ]; then
            local size=$(ls -lh "$path" | awk '{print $5}')
            echo -e "  ${GREEN}✓${NC} $desc: $size"
        else
            echo -e "  ${RED}✗${NC} $desc: not built"
        fi
    done
    
    echo ""
    echo "Swift Bindings:"
    if [ -d "generated" ] && [ "$(ls -A generated/*.swift 2>/dev/null)" ]; then
        for f in generated/*.swift; do
            echo -e "  ${GREEN}✓${NC} $(basename $f)"
        done
    else
        echo -e "  ${YELLOW}Not generated${NC}"
    fi
    
    echo ""
}

# Main execution
main() {
    local command=${1:-all}
    
    case $command in
        check)
            check_requirements
            ;;
        targets)
            install_targets
            ;;
        ios)
            check_requirements
            build_ios
            create_ios_sim_universal
            ;;
        macos)
            check_requirements
            build_macos
            create_macos_universal
            ;;
        bindings)
            generate_bindings
            ;;
        xcframework)
            create_xcframework
            ;;
        copy)
            copy_to_projects
            ;;
        all)
            check_requirements
            install_targets
            build_ios
            build_macos
            create_ios_sim_universal
            create_macos_universal
            generate_bindings
            copy_to_projects
            print_summary
            ;;
        summary)
            print_summary
            ;;
        clean)
            echo "Cleaning build artifacts..."
            cargo clean
            rm -rf generated
            rm -rf VoyageCore.xcframework
            echo -e "${GREEN}Cleaned${NC}"
            ;;
        *)
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  check       - Check build requirements"
            echo "  targets     - Install required Rust targets"
            echo "  ios         - Build iOS targets only"
            echo "  macos       - Build macOS targets only"
            echo "  bindings    - Generate Swift bindings"
            echo "  xcframework - Create XCFramework"
            echo "  copy        - Copy outputs to Xcode projects"
            echo "  all         - Build everything (default)"
            echo "  summary     - Print build summary"
            echo "  clean       - Clean build artifacts"
            ;;
    esac
}

# Run main with all arguments
main "$@"
