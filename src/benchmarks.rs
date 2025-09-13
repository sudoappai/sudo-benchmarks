use anyhow::Result;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info};

use crate::client::SudoClient;
use crate::metrics::{MetricsCollector, ThroughputStats};
use crate::models::ChatCompletionRequest;

#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub requests: Option<usize>,
    pub concurrency: usize,
    pub model: Vec<String>,
    pub streaming: bool,
}

impl BenchmarkConfig {
    pub fn latency(requests: usize, concurrency: usize, model: Vec<String>, streaming: bool) -> Self {
        Self {
            requests: Some(requests),
            concurrency,
            model,
            streaming,
        }
    }

    pub fn throughput(concurrency: usize, model: Vec<String>) -> Self {
        Self {
            requests: Some(concurrency), // Each worker makes one request
            concurrency,
            model,
            streaming: true,
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
            // Warm up the model to avoid cold-start and connection pool effects
            self.warm_up_model(&model, config.streaming).await;
            
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
                
                let request = ChatCompletionRequest::benchmark_latency_request(&model, false);
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
                
                let request = ChatCompletionRequest::benchmark_latency_request(&model, true);
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

        let test_mode = "streaming";
        info!("Running streaming throughput benchmark with {} concurrent requests per model on {} models", 
              config.concurrency, models_to_test.len());

        let mut all_results = HashMap::new();

        for model in models_to_test {
            info!("Testing {} throughput for model: {}", test_mode, model);
            // Warm up the model to avoid cold-start and connection pool effects
            self.warm_up_model(&model, config.streaming).await;
            
            let result = self.run_streaming_throughput_test(&model, config.concurrency).await;

            match result {
                Ok(stats) => {
                    all_results.insert(model.clone(), stats);
                }
                Err(e) => {
                    error!("Failed to benchmark {} throughput for {}: {}", test_mode, model, e);
                }
            }
        }

        self.print_throughput_results(all_results);
        Ok(())
    }

    async fn run_streaming_throughput_test(&self, model: &str, concurrency: usize) -> Result<ThroughputStats> {
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut collector = MetricsCollector::new();
        let mut tasks = Vec::new();

        info!("Running {} concurrent single-request streaming throughput tests for model: {}", concurrency, model);

        // Each worker makes exactly one streaming request to measure per-request TPS
        for _ in 0..concurrency {
            let client = Arc::clone(&self.client);
            let semaphore = Arc::clone(&semaphore);
            let model = model.to_string();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                let request = ChatCompletionRequest::benchmark_throughput_request(&model, true);
                client.single_request_streaming_throughput_test(&request).await
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
            .ok_or_else(|| anyhow::anyhow!("No successful streaming throughput tests for model {}", model))
    }

    // Perform a small number of warm-up requests to prime the model and connection pool.
    async fn warm_up_model(&self, model: &str, streaming: bool) {
        const WARMUPS: usize = 2;
        for _ in 0..WARMUPS {
            let req = ChatCompletionRequest::benchmark_latency_request(model, streaming);
            if streaming {
                if let Err(e) = self.client.create_streaming_chat_completion(&req).await {
                    error!("Warm-up streaming request failed for {}: {}", model, e);
                }
            } else {
                if let Err(e) = self.client.create_chat_completion(&req).await.map(|_| ()) {
                    error!("Warm-up request failed for {}: {}", model, e);
                }
            }
        }
    }

    pub async fn run_comprehensive_benchmark(
        &self,
        latency_requests: usize,
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
        let throughput_config = BenchmarkConfig::throughput(concurrency, vec![]);
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
        let test_type = "Streaming Throughput";
        println!("\n{} Benchmark Results", test_type);
        println!("{}", "=".repeat(60));

        for (model, stats) in results {
            println!("\n🤖 Model: {}", model);
            println!("─────────────────────────────");
            println!("Test Type: {}", test_type);
            println!("Concurrent Requests: {}", stats.total_requests);
            println!("Successful Requests: {}", stats.successful_requests);
            println!("Failed Requests: {}", stats.failed_requests);
            println!("Success Rate: {:.1}%", stats.success_rate);
            println!("Average Request Duration: {:?}", stats.test_duration);
            println!("Average Tokens per Second (pure generation): {:.2}", stats.mean_tokens_per_second);
        }
    }
}
