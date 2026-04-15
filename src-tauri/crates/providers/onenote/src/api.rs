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

    /// Fetch all pages from all sections of all notebooks.
    /// Returns tuples of (Page, section_display_name).
    pub async fn list_all_pages(
        &self,
        access_token: &str,
    ) -> Result<Vec<(Page, String)>, ProviderError> {
        // 1. List notebooks
        let notebooks = self.list_all::<Notebook>(
            access_token,
            &format!("{GRAPH_BASE}/notebooks"),
        ).await?;

        let mut all_pages = Vec::new();

        for nb in &notebooks {
            let sections = self.list_all::<Section>(
                access_token,
                &format!("{GRAPH_BASE}/notebooks/{}/sections", nb.id),
            ).await?;

            for section in &sections {
                let pages = self.list_all::<Page>(
                    access_token,
                    &format!("{GRAPH_BASE}/sections/{}/pages", section.id),
                ).await?;

                debug!(
                    notebook = %nb.display_name,
                    section = %section.display_name,
                    count = pages.len(),
                    "Fetched pages"
                );

                for page in pages {
                    all_pages.push((page, section.display_name.clone()));
                }
            }
        }

        Ok(all_pages)
    }

    /// Fetch the HTML content of a single page.
    pub async fn get_page_content(
        &self,
        access_token: &str,
        page_id: &str,
    ) -> Result<String, ProviderError> {
        let resp = self
            .http
            .get(format!("{GRAPH_BASE}/pages/{page_id}/content"))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("OneNote content request failed: {e}")))?;

        check_status(&resp)?;

        resp.text()
            .await
            .map_err(|e| ProviderError::Api(format!("OneNote content parse error: {e}")))
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
            "<!DOCTYPE html><html><head><title>{title}</title></head><body>{body_html}</body></html>",
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
    pub async fn update_page(
        &self,
        access_token: &str,
        page_id: &str,
        title: Option<&str>,
        body_html: &str,
    ) -> Result<(), ProviderError> {
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
        Ok(())
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
            let resp = self
                .http
                .get(&current_url)
                .bearer_auth(access_token)
                .query(&[("$top", "100")])
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("OneNote list request failed: {e}")))?;

            check_status(&resp)?;

            let page: ODataList<T> = resp
                .json()
                .await
                .map_err(|e| ProviderError::Api(format!("OneNote list parse error: {e}")))?;

            all.extend(page.value);
            next_url = page.next_link;
        }

        Ok(all)
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

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
