# SuperTinyWasmLLM - Lightweight LLM Inference with WebAssembly
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install WasmEdge with WASI-NN GGML plugin
RUN curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | \
    bash -s -- --plugins wasi_nn-ggml

# Set up environment
ENV PATH="/root/.wasmedge/bin:${PATH}"
ENV WASMEDGE_PLUGIN_PATH="/root/.wasmedge/plugin"

# Create app directory
WORKDIR /app

# Copy WASM binary
COPY supertinywasmllm.wasm /app/

# Copy example model download script
COPY download_model.sh /app/
RUN chmod +x /app/download_model.sh

# Download default model (TinyLlama 1.1B)
RUN /app/download_model.sh

# Expose model path
ENV SUPERTINYWASMLLM_MODEL_PATH="/app/model.gguf"

# Default command - run with stdin/stdout
CMD ["wasmedge", "--dir", ".:/", "--nn-preload", "default:GGML:AUTO:/app/model.gguf", "/app/supertinywasmllm.wasm"] 