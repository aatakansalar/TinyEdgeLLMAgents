# TinyEdgeLLMAgents

**Experimental Local LLM Agent Runtime**

TinyEdgeLLMAgents is the start of a research / hobby project exploring whether autonomous AI agents can run effectively on local devices without cloud dependencies. The project combines local LLM inference with modular tool execution to test the practical feasibility of edge-based AI agents.

## What It Does

TinyEdgeLLMAgents provides:
- Local LLM inference using WASI-NN and GGUF models
- Task-oriented agent interaction (not conversational chat)
- Modular tool system for mathematics, HTTP requests, and shell operations
- WebAssembly-based tool isolation for security
- JSON and CLI interfaces for integration

The system follows a simple pattern: user provides a task, the LLM plans which tools to use, tools execute, and results are returned.

## Current Implementation

**Working Components:**
- Core LLM inference engine (tested with TinyLlama-1.1B)
- Agent memory and task planning
- Tool dispatcher with automatic discovery
- Mathematical calculations, HTTP requests, basic shell operations
- CLI with multiple interaction modes

**Current Limitations:**
- Requires ~667MB model file for reasonable performance (to be worked on)
- Task-oriented interface only for now (no free-form conversation)
- Limited reasoning capabilities compared to larger models
- No streaming responses or advanced agent features yet

## Usage

```bash
# Build the project
cargo build --release

# Execute a calculation task
./target/release/tinyedgellmagents task "Calculate 15*8"

# Interactive mode
./target/release/tinyedgellmagents interactive

# Check system status
./target/release/tinyedgellmagents status
```

## Technical Architecture

```
Task Input → LLM Planning → Tool Selection → Execution → Results
```

- **Language**: Rust for performance and WebAssembly compatibility
- **LLM Backend**: WASI-NN with GGUF model support  
- **Tool Isolation**: WebAssembly sandboxing
- **Interface**: CLI with JSON I/O, designed for programmatic use

## Goals and Development

TinyEdgeLLMAgents was created to explore fundamental questions about local AI:
- Can useful AI agents operate without cloud services?
- What are the practical constraints of edge-based reasoning?
- How small can effective agent systems become?

**Current Development Focus:**
- Improving LLM response quality and consistency
- Expanding the tool ecosystem
- Better error handling and system robustness
- Documentation and examples

**Future Directions:**
The project may expand to include:
- Web-based interface for easier interaction
- Document indexing and retrieval (RAG) capabilities
- More conversational interaction patterns
- Plugin system for community-contributed tools
- Mobile and embedded device deployment

## Installation Requirements

- Rust toolchain (1.70+)
- Compatible GGUF model file (place at `core/model.gguf`)
- ~1GB disk space for model and binaries

## Project Status

TinyEdgeLLMAgents is not production software. The core functionality works but the system is designed for exploration and learning for me rather than deployment. 

Local AI systems represent an interesting alternative to cloud-based services, particularly for privacy-sensitive applications or environments with limited connectivity. This project aims to understand what's currently possible and identify areas for improvement.

## Contributing

The project welcomes feedback, testing, and contributions. Areas where input would be valuable:
- Testing with different model types and sizes
- Additional tool implementations
- Performance optimization suggestions
- Real-world usage scenarios and requirements

## License

MIT License - see [LICENSE](LICENSE) for details.
