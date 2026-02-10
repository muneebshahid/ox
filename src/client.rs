use anyhow::Result;
use reqwest::Response;
use std::time::Duration;

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;
const MAX_RETRY_DELAY_MS: u64 = 60_000;
const RETRYABLE_ERROR_CODES: [u16; 5] = [429, 500, 502, 503, 504];

fn is_retryable(status: reqwest::StatusCode) -> bool {
    RETRYABLE_ERROR_CODES.contains(&status.as_u16())
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get("retry-after")
        .and_then(|val| val.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&secs| secs > 0.0)
        .map(Duration::from_secs_f64)
}

pub async fn call_with_retry<F, Fut>(f: F) -> Result<Response>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<Response, reqwest::Error>>,
{
    let mut last_error = None;

    for attempt in 0..=MAX_RETRIES {
        match f().await {
            Ok(response) if is_retryable(response.status()) => {
                let delay = parse_retry_after(response.headers())
                    .unwrap_or_else(|| Duration::from_millis(BASE_DELAY_MS * 2u64.pow(attempt)))
                    .min(Duration::from_millis(MAX_RETRY_DELAY_MS));
                last_error = Some(format!("status {}", response.status()));
                tokio::time::sleep(delay).await;
            }
            Ok(response) if response.status() == reqwest::StatusCode::OK => return Ok(response),
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("OpenAI API error ({status}): {body}"));
            }
            Err(e) if e.is_timeout() || e.is_connect() => {
                let delay = Duration::from_millis(BASE_DELAY_MS * 2u64.pow(attempt));
                last_error = Some(e.to_string());
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Err(anyhow::anyhow!(
        "max retries exceeded: {}",
        last_error.unwrap_or_default()
    ))
}
