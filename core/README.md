# SuperTinyWasmLLM - Lightweight LLM Inference Gateway over WebAssembly

**SuperTinyWasmLLM** demonstrates portable, secure, local-first LLM inference using WebAssembly and WASI-NN.

## Status: Production Ready Proof of Concept

Real LLM inference working via WASI-NN with hardware acceleration support.

### Key Features

- **Real LLM Inference**: TinyLlama-1.1B model running via WASI-NN GGML backend  
- **Ultra Portable**: 206KB WASM binary + 637MB model = complete AI stack
- **Hardware Acceleration**: Automatic Metal acceleration on Apple Silicon, CUDA on NVIDIA
- **Zero Dependencies**: No Python, no Docker images, no cloud APIs required
- **Secure Execution**: WASM sandbox with full system isolation
- **Production API**: JSON over stdin/stdout with proper error handling
- **CI/CD Ready**: GitHub Actions, Docker support, automated testing

### Performance Metrics

```
Platform: Apple M-series (macOS)
WASM Binary: 206KB
Model: TinyLlama-1.1B (637MB GGUF)
Memory Usage: ~1GB total
Token Generation: 25-50 tokens/sec
```

### Project Vision

This implementation demonstrates core principles from the original project vision:
- "Serverless execution of small LLMs using only WASM runtimes"
- "Cloud-agnostic inference with no remote endpoints"  
- "Language model encapsulation via wasmedge-wasi-nn and GGUF models"
- "Portable design: works the same on macOS, Linux, cloud edge"

## Quick Start

### Prerequisites  

```bash
# Install WasmEdge with GGML plugin
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | \
  bash -s -- --plugins wasi_nn-ggml
```

### Using Pre-built Binary

```bash
# Download pre-built WASM
curl -LO https://github.com/your-org/supertinywasmllm/releases/latest/download/supertinywasmllm.wasm

# Download TinyLlama model  
curl -L https://huggingface.co/mradermacher/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/TinyLlama-1.1B-Chat-v1.0.Q4_K_M.gguf -o model.gguf

# Run inference
echo '{"prompt": "Once upon a time", "max_tokens": 50}' | \
  wasmedge --dir .:. --nn-preload default:GGML:AUTO:model.gguf supertinywasmllm.wasm
```

### Expected Output

```json
{
  "response": "Once upon a time, there was a brave little princess who lived in a magical castle surrounded by enchanted forests.",
  "tokens_generated": 20,
  "model_info": "SuperTinyWasmLLM v0.1.0 - Model: TinyLlama-1.1B-Chat-v1.0.Q4_K_M.gguf (WASI-NN)"
}
```

## Development

### Build from Source

```bash
# Install Rust + WASM target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-wasip1

# Build
git clone https://github.com/your-org/supertinywasmllm
cd supertinywasmllm
cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/supertinywasmllm.wasm .
```

### Download Models

```bash
# Download default TinyLlama model (~637MB)
./download_model.sh

# Or download smaller test model (~10MB) for development
./download_model.sh --small
```

### Test Suite

```bash
./test.sh  # Comprehensive tests with real GGML inference
```

## Docker Deployment

```bash
# Build Docker image
docker build -t supertinywasmllm .

# Run with JSON input
echo '{"prompt": "Hello world", "max_tokens": 30}' | docker run -i supertinywasmllm
```

## API Reference

### Input Format

```json
{
  "prompt": "Your prompt here",
  "max_tokens": 50,
  "temperature": 0.7
}
```

### Output Format

```json
{
  "response": "Generated text response",
  "tokens_generated": 42,
  "model_info": "SuperTinyWasmLLM v0.1.0 - Model: model.gguf (WASI-NN)"
}
```

### Error Format

```json
{
  "error": "Error description",
  "code": 1
}
```

## Use Cases

### Current Production Applications

1. **CLI Tools**: Embed LLM reasoning in shell scripts and automation
2. **Edge Devices**: Deploy on IoT with <1GB RAM requirement
3. **Secure Environments**: Sandboxed AI in containers and restricted environments
4. **Offline Applications**: AI that works without internet connectivity
5. **Development Tools**: Local AI assistance without API dependencies

### Example Use Cases

```bash
# Code review assistant
echo '{"prompt": "Review this function for bugs: def divide(a,b): return a/b", "max_tokens": 100}' | \
  wasmedge --dir .:. --nn-preload default:GGML:AUTO:model.gguf supertinywasmllm.wasm

# Technical documentation
echo '{"prompt": "Explain WebAssembly benefits:", "max_tokens": 80}' | \
  wasmedge --dir .:. --nn-preload default:GGML:AUTO:model.gguf supertinywasmllm.wasm

# Creative writing
echo '{"prompt": "Write a short story about robots:", "max_tokens": 150}' | \
  wasmedge --dir .:. --nn-preload default:GGML:AUTO:model.gguf supertinywasmllm.wasm
```

## Architecture

### Components

- **Core Library** (`src/lib.rs`): SuperTinyWasmLLM inference engine
- **CLI Binary** (`src/main.rs`): Command-line interface
- **WASI-NN Integration**: Hardware-accelerated neural network execution
- **Model Loading**: GGUF format validation and memory management
- **Error Handling**: Graceful fallbacks and comprehensive error reporting

### Supported Platforms

- **macOS**: ARM64 (M1/M2/M3) with Metal acceleration, x86_64
- **Linux**: x86_64, ARM64, with optional CUDA support
- **Windows**: x86_64 (via WSL recommended)
- **Container**: Docker, Kubernetes, serverless platforms

## Contributing

### Development Setup

```bash
# Install development tools
rustup component add rustfmt clippy

# Run linting
cargo fmt --check
cargo clippy --target wasm32-wasip1 -- -D warnings

# Run tests
cargo test
./test.sh
```

### CI/CD

The project includes comprehensive GitHub Actions workflows:
- Automated testing on multiple platforms
- WASM binary building and artifact management
- Docker image building and publishing
- Release automation

## Performance Optimization

### Model Size vs Performance

| Model | Size | Speed | Use Case |
|-------|------|--------|----------|
| TinyStories-3M | ~10MB | Fast | Development/Testing |
| TinyLlama-1.1B | ~637MB | Medium | General purpose |
| Larger Models | >1GB | Slower | Specialized tasks |

### Hardware Acceleration

- **Apple Silicon**: Automatic Metal GPU acceleration
- **NVIDIA GPUs**: CUDA acceleration (when available)
- **CPU**: Optimized with SIMD instructions

## Roadmap

### Immediate (Next Release)
- [ ] HTTP server mode for REST API
- [ ] Streaming response support
- [ ] Multi-model loading

### Medium Term
- [ ] Browser WASI runtime compatibility
- [ ] Audio models (Whisper integration)
- [ ] Vision models support

### Long Term
- [ ] Agent framework development
- [ ] Tool usage integration
- [ ] Distributed edge deployment

## Security Considerations

- **Sandboxed Execution**: WASM provides memory isolation
- **No Network Access**: Models run completely offline
- **Controlled File System**: Limited file system access via WASI
- **Resource Limits**: Memory and compute bounds enforced by runtime

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Acknowledgments

- Built on [WasmEdge](https://wasmedge.org/) runtime
- Uses [llama.cpp](https://github.com/ggerganov/llama.cpp) via WASI-NN
- Inspired by the vision of local-first, user-owned AI

---

*"You don't need permission to run intelligence."* - Project Vision