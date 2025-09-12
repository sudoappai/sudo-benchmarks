# Sudo API Benchmarks Project

This directory contains a comprehensive Rust-based benchmarking tool for measuring Sudo API performance across all supported models. The benchmarks are designed to generate competitive performance data for marketing and technical evaluation purposes.

## Project Structure

```
benchmarks/
├── Cargo.toml          # Rust project configuration with dependencies
├── README.md           # User-facing documentation and usage guide
├── CLAUDE.md           # This file - agent documentation
├── src/
│   ├── main.rs         # CLI entry point and command handling
│   ├── client.rs       # HTTP client for Sudo API requests
│   ├── models.rs       # API request/response data structures
│   ├── metrics.rs      # Performance metrics collection and analysis
│   └── benchmarks.rs   # Core benchmarking logic and orchestration
└── target/             # Compiled binaries (created by cargo build)
```

## Architecture Overview

### Core Components

1. **CLI Interface (`main.rs`)**
   - Uses `clap` for command-line argument parsing
   - Supports: `latency`, `throughput`, `models`, `all` commands
   - Handles environment variable loading (`.env` support)
   - **NEW**: Multiple model selection via comma-separated values or multiple `--model` flags

2. **HTTP Client (`client.rs`)**
   - `SudoClient` struct wraps `reqwest::Client`
   - Handles authentication via Bearer tokens
   - Supports both regular HTTP and streaming SSE requests
   - Implements retry logic and error handling

3. **Data Models (`models.rs`)**
   - OpenAI-compatible API structures for requests/responses
   - **IMPORTANT**: `SupportedModel` struct may need updates to match Sudo's actual API response format
   - Supports chat completions, image generation, and model listing

4. **Metrics System (`metrics.rs`)**
   - `MetricsCollector` aggregates performance data
   - Statistical analysis using `hdrhistogram` for percentiles
   - Separate metric types: `LatencyMetric`, `StreamingMetric`, `ThroughputMetric`

5. **Benchmark Runner (`benchmarks.rs`)**
   - Orchestrates concurrent test execution
   - Uses `tokio::sync::Semaphore` for concurrency control
   - Aggregates results across multiple models
   - **NEW**: Enhanced to handle multiple model selection and validation

## Key Features

### Benchmark Types
- **Regular Latency**: End-to-end HTTP request timing
- **Streaming Latency**: Time-to-first-chunk for SSE streams
- **Throughput**: Requests per second and tokens per second over time

### Performance Metrics
- P50, P95, P99 latencies
- Time to First Byte (TTFB)
- Time to First Chunk (TTFC) for streaming
- Error rates and success percentages
- Token generation rates

### Configuration
- Environment variables: `SUDO_API_KEY`, `SUDO_API_BASE_URL`
- Configurable concurrency levels
- **NEW**: Flexible model selection:
  - Single model: `--model "gpt-4o"`
  - Multiple models (comma-separated): `--model "gpt-4o,gpt-4o-mini"`
  - Multiple models (multiple flags): `--model "gpt-4o" --model "claude-3-5-sonnet-20241022"`
  - All models (default when no `--model` specified)
- Adjustable request counts and test durations

## Usage Patterns

### Environment Setup
```bash
# Required
export SUDO_API_KEY="your_api_key_here"

# Optional (defaults to production)
export SUDO_API_BASE_URL="https://sudoapp.dev/api"
```

### Common Commands
```bash
# Build the tool
cargo build --release

# Run comprehensive benchmarks
./target/release/bench all

# Test specific scenarios
./target/release/bench latency --requests 50 --concurrency 5 --streaming
./target/release/bench throughput --duration 30 --model "gpt-4o"

# Multiple model selection (NEW)
./target/release/bench latency --model "gpt-4o,gpt-4o-mini" --requests 50
./target/release/bench latency --model "gpt-4o" --model "claude-3-5-sonnet-20241022" --requests 50
./target/release/bench throughput --model "gpt-4o,gpt-4o-mini" --duration 30
```

## Technical Implementation Details

### Concurrency Model
- Uses `Arc<Semaphore>` to limit concurrent requests
- Prevents API rate limiting while maximizing throughput
- Each test maintains configurable concurrency levels

### Streaming Implementation
- Uses `eventsource-stream` crate for SSE parsing
- Tracks first chunk timing for latency measurements
- Estimates token counts from content length (4 chars ≈ 1 token)

### Error Handling
- Comprehensive error propagation using `anyhow`
- API errors are captured and included in metrics
- Network timeouts handled gracefully

### Statistical Analysis
- Uses `hdrhistogram` for accurate percentile calculations
- Maintains separate histograms per model
- Calculates means, mins, maxs across all measurements

## Known Issues and Considerations

### API Compatibility
- The `SupportedModel` struct in `models.rs` may need updates to match Sudo's actual API response
- Current structure assumes OpenAI-compatible model listing format
- **Action Item**: Verify actual `/v1/models` response structure and update accordingly

### Streaming Challenges
- SSE parsing can be sensitive to response format variations
- Token estimation is approximate (based on character count)
- Network issues can interrupt streaming connections

### Rate Limiting
- High concurrency may trigger API rate limits
- Tool respects configured concurrency but doesn't implement backoff
- Users should adjust concurrency based on their API plan limits

## Maintenance and Updates

### Adding New Benchmark Types
1. Add new metric struct to `metrics.rs`
2. Implement collection logic in `client.rs`
3. Add processing in `benchmarks.rs`
4. Update CLI interface in `main.rs`

### Supporting New API Endpoints
1. Add request/response models to `models.rs`
2. Implement client methods in `client.rs`
3. Create benchmark runners in `benchmarks.rs`

### Debugging Performance Issues
- Enable debug logging: `RUST_LOG=debug ./target/release/bench`
- Reduce concurrency to isolate issues: `--concurrency 1`
- Test individual models: `--model "specific-model-name"`

## Dependencies

### Core Runtime
- `tokio`: Async runtime and concurrency
- `reqwest`: HTTP client with streaming support
- `futures`: Stream processing utilities

### CLI and Configuration
- `clap`: Command-line argument parsing
- `dotenvy`: Environment variable loading
- `tracing`: Structured logging

### Data Processing
- `serde`: JSON serialization/deserialization
- `hdrhistogram`: Statistical analysis
- `eventsource-stream`: SSE parsing

### Utilities
- `anyhow`: Error handling
- `chrono`: Time/date handling
- `rand`: Random data generation

## Future Enhancements

### Potential Improvements
1. **WebSocket Support**: For real-time streaming benchmarks
2. **Custom Prompts**: User-defined test scenarios
3. **Result Export**: JSON/CSV output for analysis tools
4. **Comparison Mode**: Direct benchmarking against competitors
5. **Dashboard**: Web UI for result visualization
6. **Automated Scheduling**: Periodic benchmark execution

### Performance Optimizations
1. **Connection Pooling**: Reuse HTTP connections
2. **Batch Requests**: Group related API calls
3. **Memory Optimization**: Stream large responses
4. **Metrics Storage**: Persistent result history

## Development Workflow

### Testing Changes
1. Make code changes
2. `cargo check` for compilation
3. `cargo build --release` for optimized build
4. Test with minimal load first: `--requests 5 --concurrency 1`
5. Scale up for full benchmarks

### Adding New Models
- Models are fetched dynamically from `/v1/models` endpoint
- No code changes needed for new model support
- Verify model names match API response format
- **NEW**: All new models automatically supported by the multiple selection feature

This tool provides a solid foundation for comprehensive API performance analysis and can be extended as Sudo's API capabilities grow.