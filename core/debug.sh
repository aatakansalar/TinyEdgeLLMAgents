#!/bin/bash

echo "SuperTinyWasmLLM Debug Information"
echo "=================================="

# Color output for better readability
if [ -t 1 ] && command -v tput > /dev/null && tput setaf 1 > /dev/null 2>&1; then
    RED=$(tput setaf 1)
    GREEN=$(tput setaf 2)
    YELLOW=$(tput setaf 3)
    BLUE=$(tput setaf 4)
    RESET=$(tput sgr0)
else
    RED=""
    GREEN=""
    YELLOW=""
    BLUE=""
    RESET=""
fi

# System info
echo "${BLUE}System Information:${RESET}"
uname -a
echo

# Rust info
echo "${BLUE}Rust Toolchain:${RESET}"
rustc --version
cargo --version
rustup show
echo

# WasmEdge info
echo "${BLUE}WasmEdge Runtime:${RESET}"
if command -v wasmedge > /dev/null; then
    wasmedge --version
    echo "Available plugins:"
    wasmedge --help | grep -A 10 "plugin" || echo "  No plugin information available"
    echo
    
    # Check WASI-NN specifically
    if wasmedge --help | grep -q wasi_nn; then
        echo "${GREEN}[PASS]${RESET} WASI-NN plugin: FOUND"
    else
        echo "${RED}[FAIL]${RESET} WASI-NN plugin: NOT FOUND"
        echo "Install with: curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugins wasi_nn-ggml"
    fi
else
    echo "${RED}[FAIL]${RESET} WasmEdge not found in PATH"
    echo "Install with: curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugins wasi_nn-ggml"
fi
echo

# Project files
echo "${BLUE}Project Files:${RESET}"
ls -la
echo

# Build artifacts
echo "${BLUE}Build Artifacts:${RESET}"
if [ -f "supertinywasmllm.wasm" ]; then
    size=$(ls -lh supertinywasmllm.wasm | awk '{print $5}')
    echo "${GREEN}[FOUND]${RESET} supertinywasmllm.wasm: $size"
    
    # WASM file info
    if command -v file > /dev/null; then
        file_type=$(file supertinywasmllm.wasm)
        echo "File type: $file_type"
    fi
else
    echo "${RED}[MISSING]${RESET} supertinywasmllm.wasm not found"
    echo "Build with: ./build.sh or cargo build --target wasm32-wasip1 --release"
fi
echo

# Model files
echo "${BLUE}Model Files:${RESET}"
if [ -f "model.gguf" ]; then
    size=$(ls -lh model.gguf | awk '{print $5}')
    echo "${GREEN}[FOUND]${RESET} model.gguf: $size"
    
    if command -v file > /dev/null; then
        file_type=$(file model.gguf)
        echo "File type: $file_type"
    fi
else
    echo "${RED}[MISSING]${RESET} model.gguf not found"
    echo "Download with: ./download_model.sh"
fi
echo

# Environment variables
echo "${BLUE}Environment:${RESET}"
echo "SUPERTINYWASMLLM_MODEL_PATH: ${SUPERTINYWASMLLM_MODEL_PATH:-not set}"
echo "WASMEDGE_PLUGIN_PATH: ${WASMEDGE_PLUGIN_PATH:-not set}"
echo "PATH: $PATH"
echo

# Configuration check
echo "${BLUE}Configuration Check:${RESET}"
all_good=true

if [ ! -f "supertinywasmllm.wasm" ]; then
    echo "${RED}[FAIL]${RESET} Missing WASM binary"
    all_good=false
fi

if [ ! -f "model.gguf" ]; then
    echo "${RED}[FAIL]${RESET} Missing model file"
    all_good=false
fi

if ! command -v wasmedge > /dev/null; then
    echo "${RED}[FAIL]${RESET} WasmEdge not installed"
    all_good=false
fi

if [ "$all_good" = true ]; then
    echo "${GREEN}[PASS]${RESET} All prerequisites met"
else
    echo "${YELLOW}[WARN]${RESET} Some prerequisites missing"
fi
echo

# Quick functionality test
if [ -f "supertinywasmllm.wasm" ] && [ -f "model.gguf" ] && command -v wasmedge > /dev/null; then
    echo "${BLUE}Quick Test:${RESET}"
    echo "Testing basic JSON parsing..."
    
    # Test with timeout to prevent hanging
    if timeout 15s bash -c 'echo "{\"prompt\": \"test\", \"max_tokens\": 5}" | SUPERTINYWASMLLM_MODEL_PATH=model.gguf wasmedge --dir .:. supertinywasmllm.wasm' 2>&1 | head -5; then
        echo "${GREEN}[PASS]${RESET} Basic functionality test passed"
    else
        echo "${YELLOW}[WARN]${RESET} Basic test failed or timed out"
    fi
else
    echo "${YELLOW}[SKIP]${RESET} Quick test skipped - missing dependencies"
fi
echo

echo "Debug information collection complete."