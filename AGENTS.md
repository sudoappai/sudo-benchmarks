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

## Recent Changes (Benchmark Validity & Speed)

- Warm-ups: Before each model’s latency/throughput run, the runner performs 2 warm-up requests (streaming or non-streaming to match the test). This mitigates cold-start and connection-pool effects to surface best steady-state performance.
- Request profiles:
  - Latency tests now cap `max_completion_tokens` at 8 to reduce generation tail and emphasize TTFB/TTFT, yielding faster apparent responsiveness.
  - Throughput tests now use `max_completion_tokens` of 512 to better amortize overhead and produce stable tokens/sec metrics.
- Streaming token accuracy: Streaming requests now include `stream_options: { include_usage: true }` so the final stream chunk contains `usage` with `completion_tokens`; these are used when available. Otherwise, a heuristic fallback (~4 chars/token) is applied.
- Percentile correctness: P95 TTFB is computed from a sorted sample set; histogram-based percentiles for total latency remain unchanged.
- HTTP client tuning: Increased connection pool idle capacity (`pool_max_idle_per_host=32`) and idle timeout to improve reuse and reduce per-request overhead under concurrency.
- Throughput semantics: The design measures per-request TPS with one streaming request per worker. The old `--duration` flag has been removed, and throughput is always streaming. Concurrency directly controls the number of requests.

Implementation references:
- Request builders: `benchmark_latency_request` and `benchmark_throughput_request` in `models.rs`.
- Warm-up: `warm_up_model` in `benchmarks.rs` (called before each model’s tests).
- Streaming `usage` handling: `create_streaming_chat_completion` in `client.rs`.
- P95 TTFB fix: `calculate_latency_stats` in `metrics.rs`.

## Key Features

### Benchmark Types
- **Regular Latency**: End-to-end HTTP request timing
- **Streaming Latency**: Time-to-first-chunk for SSE streams
- **Regular Throughput**: Per-request token generation rate (end-to-end timing)
- **Streaming Throughput**: Per-request token generation rate (pure generation timing from first to last chunk)

### Performance Metrics
- P50, P95, P99 latencies
- Time to First Byte (TTFB)
- Time to First Chunk (TTFC) for streaming
- Error rates and success percentages
- **NEW**: Average token generation rates per request (more accurate than sustained load measurements)
- Average request duration for throughput tests

### Configuration
- Environment variables: `SUDO_API_KEY`, `SUDO_API_BASE_URL`
- Configurable concurrency levels
- **NEW**: Flexible model selection:
  - Single model: `--model "gpt-4o"`
  - Multiple models (comma-separated): `--model "gpt-4o,gpt-4o-mini"`
  - Multiple models (multiple flags): `--model "gpt-4o" --model "claude-3-5-sonnet-20241022"`
  - All models (default when no `--model` specified)
- **NEW**: Latency defaults to streaming; use `--streaming-off` to disable streaming for latency only
- Throughput is always streaming (no streaming flag)
- Adjustable request counts for latency tests
- **CHANGED**: Throughput tests now use concurrency parameter as request count (each worker makes one request)
 - Warm-ups and token caps are built-in defaults (2 warm-ups; 8 tokens for latency, 512 for throughput)

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

# Throughput benchmarks (NEW DESIGN - per-request TPS measurement)
./target/release/bench throughput --concurrency 10 --model "gpt-4o"
./target/release/bench throughput --concurrency 5 --streaming --model "gpt-4o"

# Multiple model selection
./target/release/bench latency --model "gpt-4o,gpt-4o-mini" --requests 50
./target/release/bench latency --model "gpt-4o" --model "claude-3-5-sonnet-20241022" --requests 50
./target/release/bench throughput --model "gpt-4o,grok-3" --concurrency 5 --streaming

```

## Technical Implementation Details

### Concurrency Model
- Uses `Arc<Semaphore>` to limit concurrent requests
- Prevents API rate limiting while maximizing throughput
- **CHANGED**: For throughput tests, concurrency now represents the number of concurrent single requests rather than sustained load workers

### Streaming Implementation
- Uses `eventsource-stream` crate for SSE parsing
- Tracks first chunk timing for latency measurements
- Estimates token counts from content length (4 chars ≈ 1 token)
- **NEW**: For streaming throughput, measures pure generation time (first chunk to last chunk)

### Error Handling
- Comprehensive error propagation using `anyhow`
- API errors are captured and included in metrics
- Network timeouts handled gracefully

### Statistical Analysis
- Uses `hdrhistogram` for accurate percentile calculations
- Maintains separate histograms per model  
- Calculates means, mins, maxs across all measurements
- **NEW**: For throughput tests, averages individual per-request TPS measurements rather than calculating aggregate rates

## Known Issues and Considerations

### API Compatibility
- The `SupportedModel` struct in `models.rs` may need updates to match Sudo's actual API response
- Current structure assumes OpenAI-compatible model listing format
- **Action Item**: Verify actual `/v1/models` response structure and update accordingly

### Throughput Test Design (FIXED)
- **Previous Issue**: Throughput tests were summing worker durations instead of measuring wall-clock time
- **Previous Issue**: TPS calculation was averaging individual rates instead of measuring aggregate throughput
- **FIXED**: Now measures per-request TPS and averages across concurrent requests for more accurate results
- **NEW**: Streaming vs regular throughput provides different insights (pure generation vs end-to-end)

### Streaming Challenges
- SSE parsing can be sensitive to response format variations
- Token estimation is approximate (based on character count)
- Network issues can interrupt streaming connections
- **NEW**: Streaming throughput timing depends on reliable first-chunk detection

### Rate Limiting
- High concurrency may trigger API rate limits
- Tool respects configured concurrency but doesn't implement backoff
- Users should adjust concurrency based on their API plan limits
- **NEW**: For throughput tests, consider that higher concurrency = more simultaneous requests, not longer test duration

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
