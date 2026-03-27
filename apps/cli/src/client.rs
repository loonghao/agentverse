use anyhow::{Context, Result};
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;

/// Thin HTTP client wrapping reqwest for agentverse API calls.
pub struct HubClient {
    inner: Client,
    base_url: String,
    token: Option<String>,
}

impl HubClient {
    pub fn new(base_url: &str, token: Option<&str>) -> Self {
        Self {
            inner: Client::builder()
                .user_agent(concat!("agentverse-cli/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.map(String::from),
        }
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        let req = self.inner.get(format!("{}{}", self.base_url, path));
        self.with_auth(req)
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        let req = self.inner.post(format!("{}{}", self.base_url, path));
        self.with_auth(req)
    }

    pub fn put(&self, path: &str) -> RequestBuilder {
        let req = self.inner.put(format!("{}{}", self.base_url, path));
        self.with_auth(req)
    }

    pub fn delete(&self, path: &str) -> RequestBuilder {
        let req = self.inner.delete(format!("{}{}", self.base_url, path));
        self.with_auth(req)
    }

    fn with_auth(&self, req: RequestBuilder) -> RequestBuilder {
        if let Some(tok) = &self.token {
            req.bearer_auth(tok)
        } else {
            req
        }
    }

    /// Execute a GET request and deserialize the JSON response.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.get(path).send().await.context("HTTP GET failed")?;
        let status = resp.status();
        let body = resp.text().await.context("reading response body")?;
        if !status.is_success() {
            anyhow::bail!("server returned {}: {}", status, body);
        }
        serde_json::from_str(&body).context("deserialising response")
    }

    /// Execute a POST request with a JSON body and deserialize the response.
    pub async fn post_json<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let resp = self
            .post(path)
            .json(body)
            .send()
            .await
            .context("HTTP POST failed")?;
        let status = resp.status();
        let text = resp.text().await.context("reading response body")?;
        if !status.is_success() {
            anyhow::bail!("server returned {}: {}", status, text);
        }
        serde_json::from_str(&text).context("deserialising response")
    }

    /// Execute a PUT request with a JSON body and deserialize the response.
    pub async fn put_json<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let resp = self
            .put(path)
            .json(body)
            .send()
            .await
            .context("HTTP PUT failed")?;
        let status = resp.status();
        let text = resp.text().await.context("reading response body")?;
        if !status.is_success() {
            anyhow::bail!("server returned {}: {}", status, text);
        }
        serde_json::from_str(&text).context("deserialising response")
    }

    /// Execute a DELETE request and deserialize the response (empty body OK).
    pub async fn delete_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self
            .delete(path)
            .send()
            .await
            .context("HTTP DELETE failed")?;
        let status = resp.status();
        let text = resp.text().await.context("reading response body")?;
        if !status.is_success() {
            anyhow::bail!("server returned {}: {}", status, text);
        }
        serde_json::from_str(&text).context("deserialising response")
    }
}
