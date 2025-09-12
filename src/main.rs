use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use std::env;
use tracing::info;

mod benchmarks;
mod client;
mod models;
mod metrics;

use benchmarks::{BenchmarkConfig, BenchmarkRunner};

#[derive(Parser)]
#[command(name = "sudo-benchmarks")]
#[command(about = "Performance benchmarks for Sudo API")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run latency benchmarks
    Latency {
        /// Number of requests to run
        #[arg(short, long, default_value = "100")]
        requests: usize,
        /// Number of concurrent requests
        #[arg(short, long, default_value = "10")]
        concurrency: usize,
        /// Model to benchmark (if not specified, benchmarks all models)
        #[arg(short, long)]
        model: Option<String>,
        /// Test streaming responses
        #[arg(short, long)]
        streaming: bool,
    },
    /// Run throughput benchmarks
    Throughput {
        /// Duration in seconds to run the benchmark
        #[arg(short, long, default_value = "60")]
        duration: u64,
        /// Number of concurrent requests
        #[arg(short, long, default_value = "10")]
        concurrency: usize,
        /// Model to benchmark (if not specified, benchmarks all models)
        #[arg(short, long)]
        model: Option<String>,
    },
    /// List all supported models
    Models,
    /// Run comprehensive benchmark suite
    All {
        /// Number of requests for latency tests
        #[arg(long, default_value = "50")]
        latency_requests: usize,
        /// Duration in seconds for throughput tests
        #[arg(long, default_value = "30")]
        throughput_duration: u64,
        /// Number of concurrent requests
        #[arg(short, long, default_value = "5")]
        concurrency: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Load environment variables
    if let Err(_) = dotenv() {
        info!("No .env file found, using system environment variables");
    }

    let cli = Cli::parse();

    // Get API key from environment
    let api_key = env::var("SUDO_API_KEY")
        .map_err(|_| anyhow::anyhow!("SUDO_API_KEY environment variable is required"))?;

    // Get base URL (default to production)
    let base_url = env::var("SUDO_API_BASE_URL")
        .unwrap_or_else(|_| "https://sudoapp.dev/api".to_string());

    info!("Using API base URL: {}", base_url);

    let runner = BenchmarkRunner::new(api_key, base_url).await?;

    match cli.command {
        Commands::Latency {
            requests,
            concurrency,
            model,
            streaming,
        } => {
            let config = BenchmarkConfig::latency(requests, concurrency, model, streaming);
            runner.run_latency_benchmark(config).await?;
        }
        Commands::Throughput {
            duration,
            concurrency,
            model,
        } => {
            let config = BenchmarkConfig::throughput(duration, concurrency, model);
            runner.run_throughput_benchmark(config).await?;
        }
        Commands::Models => {
            runner.list_models().await?;
        }
        Commands::All {
            latency_requests,
            throughput_duration,
            concurrency,
        } => {
            runner
                .run_comprehensive_benchmark(latency_requests, throughput_duration, concurrency)
                .await?;
        }
    }

    Ok(())
}