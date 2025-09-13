use anyhow::Result;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tracing::{debug, error};

use crate::models::{
    ChatCompletionRequest, ChatCompletionResponse, ImageGenerationRequest, ModelsResponse, StreamOptions,
};
use crate::metrics::{LatencyMetric, StreamingMetric, ThroughputMetric};

pub struct SudoClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl SudoClient {
    pub fn new(api_key: String, base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            // Encourage connection reuse and reduce setup overhead under concurrency
            .pool_max_idle_per_host(32)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url,
        }
    }

    pub async fn get_models(&self) -> Result<ModelsResponse> {
        let url = format!("{}/v1/models", self.base_url);
        
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to get models: {} - {}",
                status,
                text
            ));
        }

        let models: ModelsResponse = response.json().await?;
        Ok(models)
    }

    pub async fn create_chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<(ChatCompletionResponse, LatencyMetric)> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let start_time = Instant::now();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        let headers_received = Instant::now();

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Chat completion failed: {} - {}",
                status,
                text
            ));
        }

        let completion: ChatCompletionResponse = response.json().await?;
        let end_time = Instant::now();

        let metric = LatencyMetric {
            total_duration: end_time.duration_since(start_time),
            time_to_first_byte: headers_received.duration_since(start_time),
            request_size: serde_json::to_vec(request)?.len(),
            response_size: serde_json::to_vec(&completion)?.len(),
            model: request.model.clone(),
        };

        Ok((completion, metric))
    }

    pub async fn create_streaming_chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<StreamingMetric> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let start_time = Instant::now();

        // Create streaming request
        let mut streaming_request = request.clone();
        streaming_request.stream = Some(true);
        // Request accurate usage reporting in the stream if supported
        streaming_request.stream_options = Some(StreamOptions { include_usage: true });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&streaming_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Streaming chat completion failed: {} - {}",
                status,
                text
            ));
        }

        let mut metric = StreamingMetric {
            total_duration: Duration::new(0, 0),
            time_to_first_chunk: None,
            chunk_count: 0,
            total_tokens: 0,
            model: request.model.clone(),
            request_size: serde_json::to_vec(&streaming_request)?.len(),
        };

        // Process the streaming response
        let stream = response.bytes_stream().eventsource();
        futures::pin_mut!(stream);

        let mut first_chunk_received = false;
        let mut usage_completion_tokens: Option<u32> = None;

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    debug!("Received streaming event: type={}, data={}", event.event, event.data);
                    
                    if !first_chunk_received {
                        metric.time_to_first_chunk = Some(Instant::now().duration_since(start_time));
                        first_chunk_received = true;
                        debug!("First streaming chunk received: {:?}", metric.time_to_first_chunk);
                    }

                    metric.chunk_count += 1;

                    // Parse the event data to count tokens
                    if event.data == "[DONE]" {
                        break;
                    }

                    if let Ok(data) = serde_json::from_str::<Value>(&event.data) {
                        debug!("Parsed streaming data: {}", data);
                        // Handle the actual streaming response format from Sudo API
                        if let Some(choices) = data.get("choices").and_then(|c| c.as_array()) {
                            for choice in choices {
                                if let Some(delta) = choice.get("delta").and_then(|d| d.as_object()) {
                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                        // Rough token estimation: ~4 characters per token
                                        metric.total_tokens += (content.len() as f32 / 4.0).ceil() as u32;
                                    }
                                }
                            }
                        }
                        // Prefer precise usage if provided in a final event
                        if let Some(usage) = data.get("usage").and_then(|u| u.as_object()) {
                            if let Some(ct) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
                                usage_completion_tokens = Some(ct as u32);
                            }
                        }
                    } else {
                        debug!("Failed to parse streaming event data as JSON: {}", event.data);
                    }
                }
                Err(e) => {
                    error!("Streaming error for model {}: {}", request.model, e);
                    break;
                }
            }
        }

        metric.total_duration = Instant::now().duration_since(start_time);

        // If the server provided exact usage, use it instead of heuristic.
        if let Some(ct) = usage_completion_tokens {
            metric.total_tokens = ct;
        }

        if metric.time_to_first_chunk.is_none() {
            return Err(anyhow::anyhow!("No streaming chunks received for model {}. Chunk count: {}, Total duration: {:?}", request.model, metric.chunk_count, metric.total_duration));
        }

        Ok(metric)
    }

    #[allow(dead_code)]
    pub async fn generate_image(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<LatencyMetric> {
        let url = format!("{}/v1/images/generations", self.base_url);
        let start_time = Instant::now();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        let headers_received = Instant::now();

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Image generation failed: {} - {}",
                status,
                text
            ));
        }

        let _image_response: Value = response.json().await?;
        let end_time = Instant::now();

        let metric = LatencyMetric {
            total_duration: end_time.duration_since(start_time),
            time_to_first_byte: headers_received.duration_since(start_time),
            request_size: serde_json::to_vec(request)?.len(),
            response_size: 0, // We don't measure image response size
            model: request.model.clone(),
        };

        Ok(metric)
    }

    pub async fn single_request_throughput_test(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ThroughputMetric> {
        let start_time = Instant::now();
        
        match self.create_chat_completion(request).await {
            Ok((response, _)) => {
                let end_time = Instant::now();
                let duration = end_time.duration_since(start_time);
                
                let tokens = if let Some(usage) = response.usage {
                    usage.completion_tokens.unwrap_or(0) as f64
                } else {
                    0.0
                };

                let tokens_per_second = if duration.as_secs_f64() > 0.0 {
                    tokens / duration.as_secs_f64()
                } else {
                    0.0
                };

                Ok(ThroughputMetric {
                    duration,
                    successful_requests: 1,
                    failed_requests: 0,
                    tokens_per_second,
                    requests_per_second: 1.0 / duration.as_secs_f64(),
                    model: request.model.clone(),
                })
            }
            Err(e) => {
                let duration = Instant::now().duration_since(start_time);
                debug!("Request failed: {}", e);
                
                Ok(ThroughputMetric {
                    duration,
                    successful_requests: 0,
                    failed_requests: 1,
                    tokens_per_second: 0.0,
                    requests_per_second: 0.0,
                    model: request.model.clone(),
                })
            }
        }
    }

    pub async fn single_request_streaming_throughput_test(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ThroughputMetric> {
        match self.create_streaming_chat_completion(request).await {
            Ok(streaming_metric) => {
                // For streaming, we measure from first chunk to last chunk
                let generation_duration = if let Some(ttfc) = streaming_metric.time_to_first_chunk {
                    // Duration from first chunk to end of stream
                    streaming_metric.total_duration.saturating_sub(ttfc)
                } else {
                    // Fallback to total duration if no first chunk timing
                    streaming_metric.total_duration
                };

                let tokens_per_second = if generation_duration.as_secs_f64() > 0.0 {
                    streaming_metric.total_tokens as f64 / generation_duration.as_secs_f64()
                } else {
                    0.0
                };

                Ok(ThroughputMetric {
                    duration: generation_duration,
                    successful_requests: 1,
                    failed_requests: 0,
                    tokens_per_second,
                    // For completeness, base RPS on end-to-end duration
                    requests_per_second: if streaming_metric.total_duration.as_secs_f64() > 0.0 { 1.0 / streaming_metric.total_duration.as_secs_f64() } else { 0.0 },
                    model: request.model.clone(),
                })
            }
            Err(e) => {
                debug!("Streaming request failed: {}", e);
                
                Ok(ThroughputMetric {
                    duration: Duration::from_secs(0),
                    successful_requests: 0,
                    failed_requests: 1,
                    tokens_per_second: 0.0,
                    requests_per_second: 0.0,
                    model: request.model.clone(),
                })
            }
        }
    }
}
