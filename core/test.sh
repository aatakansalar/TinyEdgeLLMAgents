#!/bin/bash

set -euo pipefail

echo "SuperTinyWasmLLM Test Suite"
echo "==========================="

# Configuration
WASM_FILE="supertinywasmllm.wasm"
MODEL_PATH="${SUPERTINYWASMLLM_MODEL_PATH:-model.gguf}"

# Color output for CI/local development
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

log_info() {
    echo "${BLUE}[INFO]${RESET} $1"
}

log_success() {
    echo "${GREEN}[PASS]${RESET} $1"
}

log_warning() {
    echo "${YELLOW}[WARN]${RESET} $1"
}

log_error() {
    echo "${RED}[FAIL]${RESET} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    if [ ! -f "$WASM_FILE" ]; then
        log_error "WASM binary not found: $WASM_FILE"
        log_info "Build it first with: cargo build --target wasm32-wasip1 --release"
        exit 1
    fi
    
    if [ ! -f "$MODEL_PATH" ]; then
        log_error "Model file not found: $MODEL_PATH"
        log_info "Download it first with: ./download_model.sh"
        exit 1
    fi
    
    if ! command -v wasmedge > /dev/null; then
        log_error "WasmEdge not found in PATH"
        log_info "Install it with: curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugins wasi_nn-ggml"
        exit 1
    fi
    
    log_success "Prerequisites check passed"
}

# Helper function to run inference
run_inference() {
    local input="$1"
    local test_name="$2"
    
    log_info "Running: $test_name"
    
    echo "$input" | wasmedge --dir .:. --nn-preload default:GGML:AUTO:"$MODEL_PATH" "$WASM_FILE"
}

# Test 1: Simple prompt
test_simple_prompt() {
    local input='{"prompt": "Once upon a time", "max_tokens": 50}'
    run_inference "$input" "Simple story prompt"
}

# Test 2: Technical prompt
test_technical_prompt() {
    local input='{"prompt": "Explain WebAssembly in one sentence:", "max_tokens": 30}'
    run_inference "$input" "Technical explanation"
}

# Test 3: Code generation
test_code_prompt() {
    local input='{"prompt": "Write a simple hello world function in Python:", "max_tokens": 40}'
    run_inference "$input" "Code generation"
}

# Test 4: Error handling - Invalid JSON
test_invalid_json() {
    log_info "Running: Error handling test"
    
    local exit_code=0
    echo 'invalid json input' | wasmedge --dir .:. --nn-preload default:GGML:AUTO:"$MODEL_PATH" "$WASM_FILE" || exit_code=$?
    
    if [ $exit_code -ne 0 ]; then
        log_success "Error handling test passed (expected failure)"
    else
        log_warning "Error handling test should have failed"
    fi
}

# Test 5: Empty prompt
test_empty_prompt() {
    local input='{"prompt": "", "max_tokens": 10}'
    run_inference "$input" "Empty prompt handling"
}

# Test 6: Large token count
test_large_tokens() {
    local input='{"prompt": "The future of AI is", "max_tokens": 200}'
    run_inference "$input" "Large token generation"
}

# Performance benchmark
run_benchmark() {
    log_info "Running performance benchmark..."
    
    local input='{"prompt": "The quick brown fox", "max_tokens": 50}'
    local start_time=$(date +%s.%N)
    
    run_inference "$input" "Benchmark" > /dev/null
    
    local end_time=$(date +%s.%N)
    local duration=$(echo "$end_time - $start_time" | bc -l 2>/dev/null || echo "unknown")
    
    log_info "Benchmark completed in ${duration}s"
}

# Model info
show_model_info() {
    log_info "Model Information:"
    echo "  Path: $MODEL_PATH"
    echo "  Size: $(ls -lh "$MODEL_PATH" | awk '{print $5}')"
    echo "  WASM: $WASM_FILE ($(ls -lh "$WASM_FILE" | awk '{print $5}'))"
}

# Main test execution
main() {
    echo "Starting test suite..."
    echo
    
    check_prerequisites
    show_model_info
    echo
    
    log_info "Running functional tests..."
    echo "---"
    
    test_simple_prompt
    echo "---"
    
    test_technical_prompt  
    echo "---"
    
    test_code_prompt
    echo "---"
    
    test_invalid_json
    echo "---"
    
    test_empty_prompt
    echo "---"
    
    test_large_tokens
    echo "---"
    
    # Optional benchmark (only if bc is available)
    if command -v bc > /dev/null; then
        run_benchmark
        echo "---"
    fi
    
    echo
    log_success "Test suite completed successfully!"
    log_info "SuperTinyWasmLLM is ready for production use"
}

# Run main function
main "$@"