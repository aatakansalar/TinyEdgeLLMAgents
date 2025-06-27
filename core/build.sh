#!/bin/bash

set -euo pipefail

echo "SuperTinyWasmLLM Build Script"
echo "============================="

# Configuration
TARGET="wasm32-wasip1"
BUILD_TYPE="${BUILD_TYPE:-release}"
WASM_OUTPUT="supertinywasmllm.wasm"

# Color output for UX
if [ -t 1 ] && command -v tput > /dev/null && tput setaf 1 > /dev/null 2>&1; then
    GREEN=$(tput setaf 2)
    BLUE=$(tput setaf 4)
    YELLOW=$(tput setaf 3)
    RESET=$(tput sgr0)
else
    GREEN=""
    BLUE=""
    YELLOW=""
    RESET=""
fi

log_info() {
    echo "${BLUE}[INFO]${RESET} $1"
}

log_success() {
    echo "${GREEN}[SUCCESS]${RESET} $1"
}

log_warning() {
    echo "${YELLOW}[WARNING]${RESET} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking build prerequisites..."
    
    if ! command -v cargo > /dev/null; then
        echo "Error: Rust/Cargo not found. Install with:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    if ! rustup target list --installed | grep -q "$TARGET"; then
        log_info "Installing WASM target..."
        rustup target add "$TARGET"
    fi
    
    log_success "Prerequisites check passed"
}

# Build the project
build_project() {
    log_info "Building SuperTinyWasmLLM for $TARGET ($BUILD_TYPE mode)..."
    
    # Clean previous builds
    log_info "Cleaning previous builds..."
    cargo clean
    
    if [ "$BUILD_TYPE" = "release" ]; then
        cargo build --target "$TARGET" --release
        SOURCE_PATH="target/$TARGET/release/supertinywasmllm.wasm"
    else
        cargo build --target "$TARGET"
        SOURCE_PATH="target/$TARGET/debug/supertinywasmllm.wasm"
    fi
    
    if [ ! -f "$SOURCE_PATH" ]; then
        echo "Error: Build failed - WASM binary not found at $SOURCE_PATH"
        exit 1
    fi
    
    # Copy to root directory
    cp "$SOURCE_PATH" "$WASM_OUTPUT"
    
    log_success "Build completed successfully"
}

# Show build information
show_build_info() {
    log_info "Build Information:"
    echo "  Target: $TARGET"
    echo "  Mode: $BUILD_TYPE"
    echo "  Output: $WASM_OUTPUT"
    
    if [ -f "$WASM_OUTPUT" ]; then
        local size=$(ls -lh "$WASM_OUTPUT" | awk '{print $5}')
        echo "  Size: $size"
        
        # Show optimization info for release builds
        if [ "$BUILD_TYPE" = "release" ]; then
            echo "  Optimizations: Size-optimized, LTO enabled, symbols stripped"
        fi
    fi
}

# Optional: Run quick validation
validate_build() {
    if [ -f "$WASM_OUTPUT" ]; then
        log_info "Validating WASM binary..."
        
        # Check if file exists and has reasonable size
        local size_bytes=$(stat -c%s "$WASM_OUTPUT" 2>/dev/null || stat -f%z "$WASM_OUTPUT" 2>/dev/null || echo "0")
        
        if [ "$size_bytes" -lt 10000 ]; then  # Less than 10KB is suspicious
            log_warning "WASM binary seems very small ($size_bytes bytes)"
        else
            log_success "WASM binary validation passed"
        fi
    fi
}

# Main execution
main() {
    check_prerequisites
    build_project
    show_build_info
    validate_build
    
    echo
    log_success "SuperTinyWasmLLM build process completed!"
    
    # Test if wasmedge is available
    if command -v wasmedge > /dev/null; then
        log_info "WasmEdge found. You can test with:"
        echo "  echo '{\"prompt\": \"Hello world\", \"max_tokens\": 20}' | \\"
        echo "    wasmedge --dir .:. --nn-preload default:GGML:AUTO:model.gguf $WASM_OUTPUT"
    else
        log_warning "WasmEdge not found in PATH. Install it to run the binary:"
        echo "  curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | \\"
        echo "    bash -s -- --plugins wasi_nn-ggml"
    fi
}

# Run main function
main "$@"