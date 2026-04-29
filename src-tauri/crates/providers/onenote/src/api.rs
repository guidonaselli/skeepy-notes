use serde::{Deserialize, Serialize};
use tracing::debug;

use skeepy_core::ProviderError;

const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0/me/onenote";

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notebook {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    pub title: String,
    pub created_date_time: String,
    pub last_modified_date_time: String,
    pub parent_section: Option<ParentRef>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentRef {
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
struct ODataList<T> {
    value: Vec<T>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
}

// ─── API Client ───────────────────────────────────────────────────────────────

pub struct OneNoteApiClient {
    http: reqwest::Client,
}

impl OneNoteApiClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("SkeepyNotes/0.1")
            .build()
            .expect("failed to build reqwest client");
        Self { http }
    }

    /// Fetch all pages across all notebooks and sections (including Quick Notes).
    /// Uses the flat /pages endpoint with $expand=parentSection to get section names.
    pub async fn list_all_pages(
        &self,
        access_token: &str,
    ) -> Result<Vec<(Page, String)>, ProviderError> {
        let url = format!("{GRAPH_BASE}/pages");
        let mut all = Vec::new();
        let mut next_url = Some(url);

        while let Some(current_url) = next_url {
            let mut last_err = ProviderError::Api("No attempts made".to_string());
            let page_list: ODataList<Page> = 'retry: {
                for attempt in 0u32..3 {
                    if attempt > 0 {
                        tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
                    }
                    let resp = self
                        .http
                        .get(&current_url)
                        .bearer_auth(access_token)
                        .query(&[("$top", "100"), ("$expand", "parentSection")])
                        .send()
                        .await
                        .map_err(|e| ProviderError::Api(format!("OneNote list request failed: {e}")))?;

                    let status = resp.status().as_u16();
                    if is_retryable(status) {
                        last_err = ProviderError::Api(format!("OneNote API error: HTTP {status}"));
                        continue;
                    }
                    check_status(&resp)?;
                    break 'retry resp
                        .json()
                        .await
                        .map_err(|e| ProviderError::Api(format!("OneNote list parse error: {e}")))?;
                }
                return Err(last_err);
            };

            debug!(count = page_list.value.len(), "Fetched pages from OneNote");

            for page in page_list.value {
                let section_name = page
                    .parent_section
                    .as_ref()
                    .map(|s| s.display_name.clone())
                    .unwrap_or_default();
                all.push((page, section_name));
            }

            next_url = page_list.next_link;
        }

        Ok(all)
    }

    /// Fetch the HTML content of a single page.
    pub async fn get_page_content(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<String, ProviderError> {
        let url = format!("{GRAPH_BASE}/pages/{page_id}/content");
        let mut last_err = ProviderError::Api("No attempts made".to_string());
        for attempt in 0u32..3 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
            }
            let resp = self
                .http
                .get(&url)
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("OneNote content request failed: {e}")))?;

            let status = resp.status().as_u16();
            if is_retryable(status) {
                last_err = ProviderError::Api(format!("OneNote API error: HTTP {status}"));
                continue;
            }
            check_status(&resp)?;
            return resp
                .text()
                .await
                .map_err(|e| ProviderError::Api(format!("OneNote content parse error: {e}")));
        }
        Err(last_err)
    }

    /// Create a new page in a section.
    pub async fn create_page(
        &self,
        access_token: &str,
        section_id: &str,
        title: &str,
        body_html: &str,
    ) -> Result<Page, ProviderError> {
        let html = format!(
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{title}</title></head><body>{body_html}</body></html>",
            title = html_escape(title),
            body_html = body_html
        );

        let resp = self
            .http
            .post(format!("{GRAPH_BASE}/sections/{section_id}/pages"))
            .bearer_auth(access_token)
            .header("Content-Type", "text/html")
            .body(html)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("OneNote create page request failed: {e}")))?;

        check_status(&resp)?;

        resp.json::<Page>()
            .await
            .map_err(|e| ProviderError::Api(format!("OneNote create page parse error: {e}")))
    }

    /// Update the HTML content of an existing page.
    ///
    /// Microsoft Graph uses a JSON patch-command format.
    /// Replacing `body` with `action: "replace"` rewrites the page body.
    /// Update page content and return updated Page metadata with real timestamps.
    pub async fn update_page(
        &self,
        access_token: &str,
        page_id: &str,
        title: Option<&str>,
        body_html: &str,
    ) -> Result<Page, ProviderError> {
        let mut commands: Vec<serde_json::Value> = vec![
            serde_json::json!({
                "target": "body",
                "action": "replace",
                "content": body_html
            }),
        ];

        if let Some(t) = title {
            commands.push(serde_json::json!({
                "target": "title",
                "action": "replace",
                "content": html_escape(t)
            }));
        }

        let resp = self
            .http
            .patch(format!("{GRAPH_BASE}/pages/{page_id}/content"))
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .json(&commands)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("OneNote update page request failed: {e}")))?;

        check_status(&resp)?;

        // PATCH returns 204 — fetch real lastModifiedDateTime so the local
        // updated_at matches what the next sync will see from OneNote.
        self.get_page_meta(access_token, page_id).await
    }

    pub async fn get_page_meta(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<Page, ProviderError> {
        let url = format!("{GRAPH_BASE}/pages/{page_id}");
        self.get_with_retry(&url, access_token, &[]).await
    }

    /// Delete a page.
    pub async fn delete_page(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<(), ProviderError> {
        let resp = self
            .http
            .delete(format!("{GRAPH_BASE}/pages/{page_id}"))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("OneNote delete request failed: {e}")))?;

        check_status(&resp)?;
        Ok(())
    }

    /// Get all sections across all notebooks (used when creating notes).
    pub async fn list_sections(&self, access_token: &str) -> Result<Vec<Section>, ProviderError> {
        self.list_all(access_token, &format!("{GRAPH_BASE}/sections")).await
    }

    // ── Pagination helper ──────────────────────────────────────────────────────

    async fn list_all<T: for<'de> Deserialize<'de>>(
        &self,
        access_token: &str,
        url: &str,
    ) -> Result<Vec<T>, ProviderError> {
        let mut all = Vec::new();
        let mut next_url = Some(url.to_string());

        while let Some(current_url) = next_url {
            let page: ODataList<T> = self
                .get_with_retry(&current_url, access_token, &[("$top", "100")])
                .await?;
            all.extend(page.value);
            next_url = page.next_link;
        }

        Ok(all)
    }

    async fn get_with_retry<T: for<'de> serde::de::DeserializeOwned>(
        &self,
        url: &str,
        access_token: &str,
        query: &[(&str, &str)],
    ) -> Result<T, ProviderError> {
        let mut last_err = ProviderError::Api("No attempts made".to_string());
        for attempt in 0u32..3 {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(attempt))).await;
            }
            let resp = self
                .http
                .get(url)
                .bearer_auth(access_token)
                .query(query)
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("OneNote request failed: {e}")))?;

            let status = resp.status().as_u16();
            if is_retryable(status) {
                last_err = ProviderError::Api(format!("OneNote API error: HTTP {status}"));
                continue;
            }
            check_status(&resp)?;
            return resp
                .json()
                .await
                .map_err(|e| ProviderError::Api(format!("OneNote response parse error: {e}")));
        }
        Err(last_err)
    }
}

fn check_status(resp: &reqwest::Response) -> Result<(), ProviderError> {
    let status = resp.status();
    if status.as_u16() == 401 {
        return Err(ProviderError::AuthRequired);
    }
    if !status.is_success() {
        return Err(ProviderError::Api(format!(
            "OneNote API error: HTTP {}",
            status.as_u16()
        )));
    }
    Ok(())
}

fn is_retryable(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
