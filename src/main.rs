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
        #[arg(short, long, value_delimiter = ',')]
        model: Vec<String>,
        /// Disable streaming (latency defaults to streaming)
        #[arg(long = "streaming-off")]
        streaming_off: bool,
    },
    /// Run throughput benchmarks
    Throughput {
        /// Number of concurrent requests
        #[arg(short, long, default_value = "10")]
        concurrency: usize,
        /// Model to benchmark (if not specified, benchmarks all models)
        #[arg(short, long, value_delimiter = ',')]
        model: Vec<String>,
    },
    /// List all supported models
    Models,
    /// Run comprehensive benchmark suite
    All {
        /// Number of requests for latency tests
        #[arg(long, default_value = "50")]
        latency_requests: usize,
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
    if dotenv().is_err() {
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
            streaming_off,
        } => {
            let config = BenchmarkConfig::latency(requests, concurrency, model, !streaming_off);
            runner.run_latency_benchmark(config).await?;
        }
        Commands::Throughput {
            concurrency,
            model,
        } => {
            let config = BenchmarkConfig::throughput(concurrency, model);
            runner.run_throughput_benchmark(config).await?;
        }
        Commands::Models => {
            runner.list_models().await?;
        }
        Commands::All {
            latency_requests,
            concurrency,
        } => {
            runner
                .run_comprehensive_benchmark(latency_requests, concurrency)
                .await?;
        }
    }

    Ok(())
}
