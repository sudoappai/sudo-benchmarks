# Sudo API Benchmarks

This tool provides comprehensive performance benchmarks for the Sudo API, measuring latency, throughput, and streaming performance across all supported models.

## Features

- **Latency Benchmarks**: Measure response times for regular HTTP requests
- **Streaming Benchmarks**: Measure time-to-first-chunk for Server-Sent Events (SSE) streams
- **Throughput Benchmarks**: Measure tokens per second and requests per second
- **Concurrent Testing**: Run multiple requests in parallel to simulate real-world usage
- **Multi-Model Support**: Test all supported models or focus on specific ones
- **Detailed Metrics**: P50, P95, P99 latencies, error rates, and more

## Setup

1. **Environment Variables**: Create a `.env` file in the benchmarks directory:
   ```bash
   SUDO_API_KEY=your_api_key_here
   SUDO_API_BASE_URL=https://sudoapp.dev/api  # Optional, defaults to production
   ```

2. **Build the tool**:
   ```bash
   cargo build --release
   ```

## Usage

### Quick Start - Run All Benchmarks
```bash
./target/release/bench all
```

### Latency Benchmarks

Test regular HTTP response latency:
```bash
./target/release/bench latency --requests 100 --concurrency 10
```

Test streaming response latency (time to first chunk):
```bash
./target/release/bench latency --requests 50 --concurrency 5 --streaming
```

Test specific model:
```bash
./target/release/bench latency --model "gpt-4o" --requests 50
```

Test multiple models (comma-separated):
```bash
./target/release/bench latency --model "gpt-4o,gpt-4o-mini" --requests 50
```

Test multiple models (multiple flags):
```bash
./target/release/bench latency --model "gpt-4o" --model "claude-3-5-sonnet-20241022" --requests 50
```

### Throughput Benchmarks

Run throughput test for 60 seconds:
```bash
./target/release/bench throughput --duration 60 --concurrency 10
```

Test specific model throughput:
```bash
./target/release/bench throughput --model "claude-3-5-sonnet-20241022" --duration 30
```

Test multiple models for throughput:
```bash
./target/release/bench throughput --model "gpt-4o,gpt-4o-mini" --duration 30
```

### List Available Models
```bash
./target/release/bench models
```

## Command Reference

### `latency` Command
- `--requests, -r`: Number of requests to run (default: 100)
- `--concurrency, -c`: Number of concurrent requests (default: 10) 
- `--model, -m`: Models to test (optional, tests all models if not specified)
  - Single model: `--model "gpt-4o"`
  - Multiple models (comma-separated): `--model "gpt-4o,gpt-4o-mini"`
  - Multiple models (multiple flags): `--model "gpt-4o" --model "claude-3-5-sonnet-20241022"`
- `--streaming, -s`: Test streaming responses instead of regular HTTP

### `throughput` Command  
- `--duration, -d`: Duration in seconds to run the benchmark (default: 60)
- `--concurrency, -c`: Number of concurrent requests (default: 10)
- `--model, -m`: Models to test (optional, tests subset if not specified)
  - Single model: `--model "gpt-4o"`
  - Multiple models (comma-separated): `--model "gpt-4o,gpt-4o-mini"`
  - Multiple models (multiple flags): `--model "gpt-4o" --model "claude-3-5-sonnet-20241022"`

### `all` Command
- `--latency-requests`: Number of requests for latency tests (default: 50)
- `--throughput-duration`: Duration in seconds for throughput tests (default: 30)
- `--concurrency, -c`: Number of concurrent requests (default: 5)

## Metrics Explained

### Latency Metrics
- **Total Duration**: End-to-end request time including response body download
- **Time to First Byte (TTFB)**: Time until first response headers received
- **P50/P95/P99**: 50th, 95th, and 99th percentile latencies
- **Min/Max/Mean**: Statistical measures of response times

### Streaming Metrics  
- **Time to First Chunk**: Time until first SSE chunk received (critical for perceived responsiveness)
- **Tokens per Second**: Rate of token generation during streaming
- **Chunk Count**: Number of streaming chunks received
- **Total Duration**: Complete streaming session time

### Throughput Metrics
- **Requests per Second**: Sustainable request rate
- **Tokens per Second**: Token generation rate across all requests
- **Success Rate**: Percentage of successful requests
- **Error Rate**: Percentage of failed requests

## Sample Output

```
Regular Latency Benchmark Results
=================================================================

🤖 Model: gpt-4
─────────────────────────────
LatencyStats {
    model: "gpt-4",
    request_count: 50,
    min_latency: 847ms,
    max_latency: 3421ms,
    mean_latency: 1234ms,
    p50_latency: 1156ms,
    p95_latency: 2344ms,
    p99_latency: 3102ms,
    mean_ttfb: 234ms,
    p95_ttfb: 456ms,
    error_rate: 0.0,
}
```

## Best Practices

1. **Start Small**: Begin with lower request counts and concurrency to avoid rate limiting
2. **Monitor API Limits**: Be aware of your API rate limits and adjust concurrency accordingly
3. **Multiple Runs**: Run benchmarks multiple times to account for network variability
4. **Model Selection**: Test your most important models first, then expand to others
5. **Production Testing**: Use the production API URL for realistic benchmarks

## Troubleshooting

### Rate Limiting
If you encounter 429 (Too Many Requests) errors:
- Reduce `--concurrency` parameter
- Increase time between requests
- Check your API plan limits

### Authentication Errors
- Verify `SUDO_API_KEY` is set correctly
- Ensure the API key has proper permissions
- Check that the API key hasn't expired

### Network Issues
- Try reducing concurrency for unstable connections
- Consider running benchmarks from a server closer to Sudo's infrastructure
- Monitor your internet connection stability during long-running tests

## Contributing

To add new benchmark types or metrics:
1. Add new metrics to `src/metrics.rs`
2. Implement collection logic in `src/client.rs`
3. Add processing logic in `src/benchmarks.rs`
4. Update the CLI interface in `src/main.rs`