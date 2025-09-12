use std::time::Duration;
use hdrhistogram::Histogram;

#[derive(Debug, Clone)]
pub struct LatencyMetric {
    pub total_duration: Duration,
    pub time_to_first_byte: Duration,
    #[allow(dead_code)]
    pub request_size: usize,
    #[allow(dead_code)]
    pub response_size: usize,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct StreamingMetric {
    pub total_duration: Duration,
    pub time_to_first_chunk: Option<Duration>,
    pub chunk_count: u32,
    pub total_tokens: u32,
    pub model: String,
    #[allow(dead_code)]
    pub request_size: usize,
}

#[derive(Debug, Clone)]
pub struct ThroughputMetric {
    pub duration: Duration,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub tokens_per_second: f64,
    pub requests_per_second: f64,
    pub model: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct LatencyStats {
    pub model: String,
    pub request_count: usize,
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub mean_latency: Duration,
    pub p50_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub mean_ttfb: Duration,
    pub p95_ttfb: Duration,
    pub error_rate: f64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct StreamingStats {
    pub model: String,
    pub request_count: usize,
    pub mean_time_to_first_chunk: Duration,
    pub p95_time_to_first_chunk: Duration,
    pub mean_tokens_per_second: f64,
    pub total_chunks: u32,
    pub error_rate: f64,
}

#[derive(Debug)]
pub struct ThroughputStats {
    #[allow(dead_code)]
    pub model: String,
    pub test_duration: Duration,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub mean_requests_per_second: f64,
    pub mean_tokens_per_second: f64,
    pub success_rate: f64,
}

pub struct MetricsCollector {
    latency_metrics: Vec<LatencyMetric>,
    streaming_metrics: Vec<StreamingMetric>,
    throughput_metrics: Vec<ThroughputMetric>,
    errors: Vec<String>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            latency_metrics: Vec::new(),
            streaming_metrics: Vec::new(),
            throughput_metrics: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_latency_metric(&mut self, metric: LatencyMetric) {
        self.latency_metrics.push(metric);
    }

    pub fn add_streaming_metric(&mut self, metric: StreamingMetric) {
        self.streaming_metrics.push(metric);
    }

    pub fn add_throughput_metric(&mut self, metric: ThroughputMetric) {
        self.throughput_metrics.push(metric);
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn calculate_latency_stats(&self, model: &str) -> Option<LatencyStats> {
        let model_metrics: Vec<_> = self
            .latency_metrics
            .iter()
            .filter(|m| m.model == model)
            .collect();

        if model_metrics.is_empty() {
            return None;
        }

        let mut latencies: Vec<u64> = model_metrics
            .iter()
            .map(|m| m.total_duration.as_millis() as u64)
            .collect();
        latencies.sort();

        let ttfbs: Vec<u64> = model_metrics
            .iter()
            .map(|m| m.time_to_first_byte.as_millis() as u64)
            .collect();

        let mut histogram = Histogram::<u64>::new(3).unwrap();
        for &latency in &latencies {
            histogram.record(latency).unwrap();
        }

        let mean_latency = Duration::from_millis(
            latencies.iter().sum::<u64>() / latencies.len() as u64
        );
        let mean_ttfb = Duration::from_millis(
            ttfbs.iter().sum::<u64>() / ttfbs.len() as u64
        );

        Some(LatencyStats {
            model: model.to_string(),
            request_count: model_metrics.len(),
            min_latency: Duration::from_millis(*latencies.first().unwrap()),
            max_latency: Duration::from_millis(*latencies.last().unwrap()),
            mean_latency,
            p50_latency: Duration::from_millis(histogram.value_at_quantile(0.5)),
            p95_latency: Duration::from_millis(histogram.value_at_quantile(0.95)),
            p99_latency: Duration::from_millis(histogram.value_at_quantile(0.99)),
            mean_ttfb,
            p95_ttfb: Duration::from_millis(
                ttfbs.to_vec().get((ttfbs.len() * 95 / 100).min(ttfbs.len() - 1)).copied().unwrap_or(0)
            ),
            error_rate: 0.0, // TODO: Track errors properly
        })
    }

    pub fn calculate_streaming_stats(&self, model: &str) -> Option<StreamingStats> {
        let model_metrics: Vec<_> = self
            .streaming_metrics
            .iter()
            .filter(|m| m.model == model)
            .collect();

        if model_metrics.is_empty() {
            return None;
        }

        let mut ttfcs: Vec<Duration> = model_metrics
            .iter()
            .filter_map(|m| m.time_to_first_chunk)
            .collect();
        ttfcs.sort();

        let mean_ttfc = if ttfcs.is_empty() {
            Duration::from_millis(0)
        } else {
            Duration::from_nanos(
                ttfcs.iter().map(|d| d.as_nanos() as u64).sum::<u64>() / ttfcs.len() as u64
            )
        };

        let p95_ttfc = ttfcs.get(ttfcs.len() * 95 / 100).copied().unwrap_or(Duration::from_millis(0));

        let total_tokens: u32 = model_metrics.iter().map(|m| m.total_tokens).sum();
        let total_duration: Duration = model_metrics.iter().map(|m| m.total_duration).sum();
        let mean_tokens_per_second = if total_duration.as_secs_f64() > 0.0 {
            total_tokens as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        Some(StreamingStats {
            model: model.to_string(),
            request_count: model_metrics.len(),
            mean_time_to_first_chunk: mean_ttfc,
            p95_time_to_first_chunk: p95_ttfc,
            mean_tokens_per_second,
            total_chunks: model_metrics.iter().map(|m| m.chunk_count).sum(),
            error_rate: 0.0, // TODO: Track errors properly
        })
    }

    pub fn calculate_throughput_stats(&self, model: &str) -> Option<ThroughputStats> {
        let model_metrics: Vec<_> = self
            .throughput_metrics
            .iter()
            .filter(|m| m.model == model)
            .collect();

        if model_metrics.is_empty() {
            return None;
        }

        let total_duration = model_metrics.iter().map(|m| m.duration).sum();
        let total_requests = model_metrics.iter().map(|m| m.successful_requests + m.failed_requests).sum();
        let successful_requests = model_metrics.iter().map(|m| m.successful_requests).sum();
        let failed_requests = model_metrics.iter().map(|m| m.failed_requests).sum();
        
        let mean_rps = model_metrics.iter().map(|m| m.requests_per_second).sum::<f64>() / model_metrics.len() as f64;
        let mean_tps = model_metrics.iter().map(|m| m.tokens_per_second).sum::<f64>() / model_metrics.len() as f64;

        Some(ThroughputStats {
            model: model.to_string(),
            test_duration: total_duration,
            total_requests,
            successful_requests,
            failed_requests,
            mean_requests_per_second: mean_rps,
            mean_tokens_per_second: mean_tps,
            success_rate: if total_requests > 0 { 
                successful_requests as f64 / total_requests as f64 * 100.0 
            } else { 
                0.0 
            },
        })
    }

    #[allow(dead_code)]
    pub fn get_models(&self) -> Vec<String> {
        let mut models = std::collections::HashSet::new();
        
        for metric in &self.latency_metrics {
            models.insert(metric.model.clone());
        }
        for metric in &self.streaming_metrics {
            models.insert(metric.model.clone());
        }
        for metric in &self.throughput_metrics {
            models.insert(metric.model.clone());
        }
        
        let mut model_list: Vec<String> = models.into_iter().collect();
        model_list.sort();
        model_list
    }
}