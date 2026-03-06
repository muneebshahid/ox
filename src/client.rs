const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;
const MAX_DELAY_MS: u64 = 10000;
const RETRYABLE_ERROR_CODES: [u16; 5] = [429, 500, 502, 503, 504];

#[derive(thiserror::Error, Debug)]
pub enum HTTPClientError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
}

pub async fn post(
    payload: &serde_json::Value,
    headers: Option<reqwest::header::HeaderMap>,
    url: &str,
) -> Result<serde_json::Value, HTTPClientError> {
    let client = reqwest::Client::new();
    let resolved_headers = headers.unwrap_or_default();
    for retry in 0..=MAX_RETRIES {
        let request = client
            .post(url)
            .json(payload)
            .headers(resolved_headers.clone());
        let result = request.send().await.and_then(|r| r.error_for_status());
        match result {
            Ok(res) => {
                let parsed = res.json::<serde_json::Value>().await?;
                return Ok(parsed);
            }
            Err(e) => {
                let is_retryable = is_retryable_error(&e);
                if retry == MAX_RETRIES || !is_retryable {
                    return Err(e.into());
                } else {
                    tokio::time::sleep(backoff_delay(retry)).await;
                }
            }
        }
    }
    unreachable!("This point should never be reached");
}

fn is_retryable_error(e: &reqwest::Error) -> bool {
    e.is_timeout()
        || e.is_connect()
        || e.status()
            .map_or(false, |code| RETRYABLE_ERROR_CODES.contains(&code.as_u16()))
}

fn backoff_delay(retry: u32) -> std::time::Duration {
    let delay = BASE_DELAY_MS * 2u64.pow(retry) as u64;
    std::time::Duration::from_millis(std::cmp::min(delay, MAX_DELAY_MS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_delay() {
        assert_eq!(backoff_delay(0), std::time::Duration::from_millis(1000));
        assert_eq!(backoff_delay(1), std::time::Duration::from_millis(2000));
        assert_eq!(backoff_delay(2), std::time::Duration::from_millis(4000));
        assert_eq!(backoff_delay(3), std::time::Duration::from_millis(8000));
    }

    #[test]
    fn test_backoff_delay_max() {
        assert_eq!(backoff_delay(4), std::time::Duration::from_millis(10000));
        assert_eq!(backoff_delay(10), std::time::Duration::from_millis(10000));
    }

    #[tokio::test]
    async fn test_post_sends_payload_and_headers() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .match_header("x-custom", "hello")
            .match_body(mockito::Matcher::Json(serde_json::json!({"key": "value"})))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"result": "ok"}"#)
            .create_async()
            .await;

        let payload = serde_json::json!({"key": "value"});
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-custom", "hello".parse().unwrap());

        let result = post(&payload, Some(headers), &format!("{}/test", server.url())).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["result"], "ok");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_retries_on_server_error() {
        let mut server = mockito::Server::new_async().await;
        let fail_mock = server
            .mock("POST", "/test")
            .with_status(503)
            .expect(2)
            .create_async()
            .await;
        let success_mock = server
            .mock("POST", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"retried": true}"#)
            .create_async()
            .await;

        let payload = serde_json::json!({});
        let result = post(&payload, None, &format!("{}/test", server.url())).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["retried"], true);
        fail_mock.assert_async().await;
        success_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_fails_immediately_on_client_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(400)
            .expect(1)
            .create_async()
            .await;

        let payload = serde_json::json!({});
        let result = post(&payload, None, &format!("{}/test", server.url())).await;

        assert!(result.is_err());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_returns_error_when_retries_exhausted() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(500)
            .expect((MAX_RETRIES + 1) as usize)
            .create_async()
            .await;

        let payload = serde_json::json!({});
        let result = post(&payload, None, &format!("{}/test", server.url())).await;

        assert!(result.is_err());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_returns_error_on_invalid_json() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json")
            .create_async()
            .await;

        let payload = serde_json::json!({});
        let result = post(&payload, None, &format!("{}/test", server.url())).await;

        assert!(result.is_err());
        mock.assert_async().await;
    }
}
