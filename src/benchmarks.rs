use anyhow::Result;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{error, info};

use crate::client::SudoClient;
use crate::metrics::{MetricsCollector, ThroughputStats};
use crate::models::ChatCompletionRequest;

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub requests: Option<usize>,
    pub concurrency: usize,
    pub duration: Option<Duration>,
    pub model: Vec<String>,
    pub streaming: bool,
}

impl BenchmarkConfig {
    pub fn latency(requests: usize, concurrency: usize, model: Vec<String>, streaming: bool) -> Self {
        Self {
            requests: Some(requests),
            concurrency,
            duration: None,
            model,
            streaming,
        }
    }

    pub fn throughput(duration_secs: u64, concurrency: usize, model: Vec<String>) -> Self {
        Self {
            requests: None,
            concurrency,
            duration: Some(Duration::from_secs(duration_secs)),
            model,
            streaming: false,
        }
    }
}

pub struct BenchmarkRunner {
    client: Arc<SudoClient>,
    supported_models: Vec<String>,
}

impl BenchmarkRunner {
    pub async fn new(api_key: String, base_url: String) -> Result<Self> {
        let client = Arc::new(SudoClient::new(api_key, base_url));
        
        // Fetch supported models
        let models_response = client.get_models().await?;
        let supported_models: Vec<String> = models_response
            .data
            .into_iter()
            .map(|m| m.model_name)
            .collect();

        info!("Loaded {} supported models", supported_models.len());

        Ok(Self {
            client,
            supported_models,
        })
    }

    pub async fn list_models(&self) -> Result<()> {
        println!("Supported Models:");
        println!("─────────────────");
        for (i, model) in self.supported_models.iter().enumerate() {
            println!("{}. {}", i + 1, model);
        }
        Ok(())
    }

    pub async fn run_latency_benchmark(&self, config: BenchmarkConfig) -> Result<()> {
        let models_to_test = if !config.model.is_empty() {
            // Validate all requested models are supported
            for model in &config.model {
                if !self.supported_models.contains(model) {
                    return Err(anyhow::anyhow!("Model '{}' is not supported", model));
                }
            }
            config.model.clone()
        } else {
            self.supported_models.clone()
        };

        info!("Running latency benchmark on {} models", models_to_test.len());

        let mut all_results = HashMap::new();

        for model in models_to_test {
            info!("Testing model: {}", model);
            
            let result = if config.streaming {
                self.run_streaming_latency_test(&model, config.requests.unwrap_or(50), config.concurrency).await
            } else {
                self.run_regular_latency_test(&model, config.requests.unwrap_or(50), config.concurrency).await
            };

            match result {
                Ok(stats) => {
                    all_results.insert(model.clone(), stats);
                }
                Err(e) => {
                    error!("Failed to benchmark {}: {}", model, e);
                }
            }
        }

        self.print_latency_results(all_results, config.streaming);
        Ok(())
    }

    async fn run_regular_latency_test(&self, model: &str, requests: usize, concurrency: usize) -> Result<Box<dyn std::fmt::Debug>> {
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut collector = MetricsCollector::new();
        let mut tasks = Vec::new();

        for _ in 0..requests {
            let client = Arc::clone(&self.client);
            let semaphore = Arc::clone(&semaphore);
            let model = model.to_string();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                let request = ChatCompletionRequest::benchmark_request(&model, false);
                client.create_chat_completion(&request).await
            });

            tasks.push(task);
        }

        let results = join_all(tasks).await;
        
        for result in results {
            match result {
                Ok(Ok((_, metric))) => collector.add_latency_metric(metric),
                Ok(Err(e)) => collector.add_error(e.to_string()),
                Err(e) => collector.add_error(format!("Task error: {}", e)),
            }
        }

        if let Some(stats) = collector.calculate_latency_stats(model) {
            Ok(Box::new(stats))
        } else {
            Err(anyhow::anyhow!("No successful requests for model {}", model))
        }
    }

    async fn run_streaming_latency_test(&self, model: &str, requests: usize, concurrency: usize) -> Result<Box<dyn std::fmt::Debug>> {
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut collector = MetricsCollector::new();
        let mut tasks = Vec::new();

        for _ in 0..requests {
            let client = Arc::clone(&self.client);
            let semaphore = Arc::clone(&semaphore);
            let model = model.to_string();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                let request = ChatCompletionRequest::benchmark_request(&model, true);
                client.create_streaming_chat_completion(&request).await
            });

            tasks.push(task);
        }

        let results = join_all(tasks).await;
        
        for result in results {
            match result {
                Ok(Ok(metric)) => collector.add_streaming_metric(metric),
                Ok(Err(e)) => {
                    error!("Streaming request failed for model {}: {}", model, e);
                    collector.add_error(e.to_string());
                },
                Err(e) => {
                    error!("Task error for model {}: {}", model, e);
                    collector.add_error(format!("Task error: {}", e));
                },
            }
        }

        if let Some(stats) = collector.calculate_streaming_stats(model) {
            Ok(Box::new(stats))
        } else {
            Err(anyhow::anyhow!("No successful streaming requests for model {}", model))
        }
    }

    pub async fn run_throughput_benchmark(&self, config: BenchmarkConfig) -> Result<()> {
        let models_to_test = if !config.model.is_empty() {
            // Validate all requested models are supported
            for model in &config.model {
                if !self.supported_models.contains(model) {
                    return Err(anyhow::anyhow!("Model '{}' is not supported", model));
                }
            }
            config.model.clone()
        } else {
            // For throughput tests, we'll test a subset of models to avoid overwhelming the API
            self.supported_models.iter().take(5).cloned().collect()
        };

        let duration = config.duration.unwrap_or(Duration::from_secs(60));
        info!("Running throughput benchmark for {:?} on {} models", duration, models_to_test.len());

        let mut all_results = HashMap::new();

        for model in models_to_test {
            info!("Testing throughput for model: {}", model);
            
            match self.run_throughput_test(&model, duration, config.concurrency).await {
                Ok(stats) => {
                    all_results.insert(model.clone(), stats);
                }
                Err(e) => {
                    error!("Failed to benchmark throughput for {}: {}", model, e);
                }
            }
        }

        self.print_throughput_results(all_results);
        Ok(())
    }

    async fn run_throughput_test(&self, model: &str, duration: Duration, concurrency: usize) -> Result<ThroughputStats> {
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut collector = MetricsCollector::new();
        let mut tasks = Vec::new();

        // Run concurrent throughput tests
        for _ in 0..concurrency {
            let client = Arc::clone(&self.client);
            let semaphore = Arc::clone(&semaphore);
            let model = model.to_string();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                let request = ChatCompletionRequest::benchmark_request(&model, false);
                client.throughput_test(&request, duration).await
            });

            tasks.push(task);
        }

        let results = join_all(tasks).await;
        
        for result in results {
            match result {
                Ok(Ok(metric)) => collector.add_throughput_metric(metric),
                Ok(Err(e)) => collector.add_error(e.to_string()),
                Err(e) => collector.add_error(format!("Task error: {}", e)),
            }
        }

        collector.calculate_throughput_stats(model)
            .ok_or_else(|| anyhow::anyhow!("No successful throughput tests for model {}", model))
    }

    pub async fn run_comprehensive_benchmark(
        &self,
        latency_requests: usize,
        throughput_duration: u64,
        concurrency: usize,
    ) -> Result<()> {
        info!("🚀 Starting comprehensive benchmark suite");
        println!();
        
        // Run latency benchmarks (regular)
        info!("📊 Running regular latency benchmarks...");
        let latency_config = BenchmarkConfig::latency(latency_requests, concurrency, vec![], false);
        self.run_latency_benchmark(latency_config).await?;
        
        println!("\n{}\n", "=".repeat(80));
        
        // Run streaming latency benchmarks
        info!("📡 Running streaming latency benchmarks...");
        let streaming_config = BenchmarkConfig::latency(latency_requests, concurrency, vec![], true);
        self.run_latency_benchmark(streaming_config).await?;
        
        println!("\n{}\n", "=".repeat(80));
        
        // Run throughput benchmarks
        info!("⚡ Running throughput benchmarks...");
        let throughput_config = BenchmarkConfig::throughput(throughput_duration, concurrency, vec![]);
        self.run_throughput_benchmark(throughput_config).await?;

        info!("✅ Comprehensive benchmark suite completed!");
        Ok(())
    }

    fn print_latency_results(&self, results: HashMap<String, Box<dyn std::fmt::Debug>>, streaming: bool) {
        let benchmark_type = if streaming { "Streaming Latency" } else { "Regular Latency" };
        
        println!("\n{} Benchmark Results", benchmark_type);
        println!("{}", "=".repeat(60));

        for (model, stats) in results {
            println!("\n🤖 Model: {}", model);
            println!("─────────────────────────────");
            println!("{:#?}", stats);
        }
    }

    fn print_throughput_results(&self, results: HashMap<String, ThroughputStats>) {
        println!("\nThroughput Benchmark Results");
        println!("{}", "=".repeat(60));

        for (model, stats) in results {
            println!("\n🤖 Model: {}", model);
            println!("─────────────────────────────");
            println!("Test Duration: {:?}", stats.test_duration);
            println!("Total Requests: {}", stats.total_requests);
            println!("Successful Requests: {}", stats.successful_requests);
            println!("Failed Requests: {}", stats.failed_requests);
            println!("Success Rate: {:.1}%", stats.success_rate);
            println!("Requests per Second: {:.2}", stats.mean_requests_per_second);
            println!("Tokens per Second: {:.2}", stats.mean_tokens_per_second);
        }
    }
}