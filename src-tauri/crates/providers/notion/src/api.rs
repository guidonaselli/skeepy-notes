use serde::{Deserialize, Serialize};
use tracing::debug;

use skeepy_core::ProviderError;

const API_BASE: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2022-06-28";

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct NotionPage {
    pub id: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub archived: bool,
    pub properties: serde_json::Value,
}

impl NotionPage {
    pub fn title(&self) -> Option<String> {
        // Pages have a "title" property which is an array of rich text objects.
        // The key varies by database — try common names.
        let props = self.properties.as_object()?;
        for key in &["Name", "Title", "title", "name"] {
            if let Some(prop) = props.get(*key) {
                if let Some(text) = extract_rich_text_value(prop) {
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Deserialize)]
struct PagedResponse<T> {
    results: Vec<T>,
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Block {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub has_children: bool,
    #[serde(flatten)]
    pub content: serde_json::Value,
}

// ─── API Client ───────────────────────────────────────────────────────────────

pub struct NotionApiClient {
    http: reqwest::Client,
}

impl NotionApiClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("SkeepyNotes/0.1")
            .build()
            .expect("failed to build reqwest client");
        Self { http }
    }

    fn auth_request(&self, method: reqwest::Method, url: &str, token: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(token)
            .header("Notion-Version", NOTION_VERSION)
    }

    /// Search all pages the integration has access to.
    pub async fn list_pages(&self, access_token: &str) -> Result<Vec<NotionPage>, ProviderError> {
        let mut pages = Vec::new();
        let mut start_cursor: Option<String> = None;

        loop {
            let mut body = serde_json::json!({
                "filter": { "value": "page", "property": "object" },
                "page_size": 100
            });

            if let Some(ref cursor) = start_cursor {
                body["start_cursor"] = serde_json::json!(cursor);
            }

            let resp = self
                .auth_request(reqwest::Method::POST, &format!("{API_BASE}/search"), access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("Notion search request failed: {e}")))?;

            check_status(&resp)?;

            let page_resp: PagedResponse<NotionPage> = resp
                .json()
                .await
                .map_err(|e| ProviderError::Api(format!("Notion search parse error: {e}")))?;

            debug!(count = page_resp.results.len(), "Fetched Notion pages batch");
            pages.extend(page_resp.results);

            if page_resp.has_more {
                start_cursor = page_resp.next_cursor;
            } else {
                break;
            }
        }

        Ok(pages)
    }

    /// Fetch the content blocks of a page (one level deep).
    pub async fn get_page_blocks(&self, access_token: &str, page_id: &str) -> Result<Vec<Block>, ProviderError> {
        self.list_blocks(access_token, page_id).await
    }

    async fn list_blocks(&self, access_token: &str, block_id: &str) -> Result<Vec<Block>, ProviderError> {
        let mut blocks = Vec::new();
        let mut start_cursor: Option<String> = None;

        loop {
            let mut url = format!("{API_BASE}/blocks/{block_id}/children?page_size=100");
            if let Some(ref cursor) = start_cursor {
                url.push_str(&format!("&start_cursor={cursor}"));
            }

            let resp = self
                .auth_request(reqwest::Method::GET, &url, access_token)
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("Notion blocks request failed: {e}")))?;

            check_status(&resp)?;

            let page_resp: PagedResponse<Block> = resp
                .json()
                .await
                .map_err(|e| ProviderError::Api(format!("Notion blocks parse error: {e}")))?;

            blocks.extend(page_resp.results);

            if page_resp.has_more {
                start_cursor = page_resp.next_cursor;
            } else {
                break;
            }
        }

        Ok(blocks)
    }

    /// Create a new page under the given parent (page or database).
    pub async fn create_page(
        &self,
        access_token: &str,
        parent_page_id: &str,
        title: &str,
        content: &str,
    ) -> Result<NotionPage, ProviderError> {
        let body = serde_json::json!({
            "parent": { "page_id": parent_page_id },
            "properties": {
                "title": {
                    "title": [{ "text": { "content": title } }]
                }
            },
            "children": text_to_blocks(content)
        });

        let resp = self
            .auth_request(reqwest::Method::POST, &format!("{API_BASE}/pages"), access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Notion create page request failed: {e}")))?;

        check_status(&resp)?;

        resp.json::<NotionPage>()
            .await
            .map_err(|e| ProviderError::Api(format!("Notion create page parse error: {e}")))
    }

    /// Update the title and first-level text content of a page.
    /// Notion's update API requires replacing individual blocks.
    pub async fn update_page(
        &self,
        access_token: &str,
        page_id: &str,
        title: Option<&str>,
        content: &str,
    ) -> Result<(), ProviderError> {
        // 1. Update title via page properties endpoint (if provided)
        if let Some(t) = title {
            let body = serde_json::json!({
                "properties": {
                    "title": {
                        "title": [{ "text": { "content": t } }]
                    }
                }
            });
            let resp = self
                .auth_request(reqwest::Method::PATCH, &format!("{API_BASE}/pages/{page_id}"), access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("Notion update page title failed: {e}")))?;
            check_status(&resp)?;
        }

        // 2. Get existing top-level blocks to archive them
        let existing_blocks = self.list_blocks(access_token, page_id).await?;

        // Archive all existing blocks
        for block in &existing_blocks {
            let body = serde_json::json!({ "archived": true });
            let resp = self
                .auth_request(reqwest::Method::PATCH, &format!("{API_BASE}/blocks/{}", block.id), access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("Notion archive block failed: {e}")))?;
            check_status(&resp)?;
        }

        // 3. Append new content blocks
        let new_blocks = text_to_blocks(content);
        if !new_blocks.is_empty() {
            let body = serde_json::json!({ "children": new_blocks });
            let resp = self
                .auth_request(reqwest::Method::PATCH, &format!("{API_BASE}/blocks/{page_id}/children"), access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("Notion append blocks failed: {e}")))?;
            check_status(&resp)?;
        }

        Ok(())
    }

    /// Archive (soft-delete) a page.
    pub async fn archive_page(&self, access_token: &str, page_id: &str) -> Result<(), ProviderError> {
        let body = serde_json::json!({ "archived": true });
        let resp = self
            .auth_request(reqwest::Method::PATCH, &format!("{API_BASE}/pages/{page_id}"), access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Notion archive page request failed: {e}")))?;
        check_status(&resp)?;
        Ok(())
    }
}

// ─── Content conversion ───────────────────────────────────────────────────────

/// Convert Notion blocks to plain text.
pub fn blocks_to_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        let text = block_to_text(block);
        if !text.is_empty() {
            out.push_str(&text);
            out.push('\n');
        }
    }
    out.trim_end().to_string()
}

fn block_to_text(block: &Block) -> String {
    let content = &block.content;
    let bt = block.block_type.as_str();

    // Most block types store their text as a rich_text array under the type key
    if let Some(rich_text) = content
        .get(&block.block_type)
        .and_then(|b| b.get("rich_text"))
        .and_then(|rt| rt.as_array())
    {
        let text = rich_text_array_to_string(rich_text);
        match bt {
            "bulleted_list_item" => return format!("• {text}"),
            "numbered_list_item" => return format!("1. {text}"),
            "to_do" => {
                let checked = content
                    .get("to_do")
                    .and_then(|t| t.get("checked"))
                    .and_then(|c| c.as_bool())
                    .unwrap_or(false);
                let mark = if checked { "[x]" } else { "[ ]" };
                return format!("{mark} {text}");
            }
            "heading_1" => return format!("# {text}"),
            "heading_2" => return format!("## {text}"),
            "heading_3" => return format!("### {text}"),
            "quote" => return format!("> {text}"),
            "code" => return format!("```\n{text}\n```"),
            _ => return text,
        }
    }

    String::new()
}

fn rich_text_array_to_string(arr: &[serde_json::Value]) -> String {
    arr.iter()
        .filter_map(|item| {
            item.get("plain_text")
                .and_then(|t| t.as_str())
                .map(str::to_string)
        })
        .collect()
}

fn extract_rich_text_value(prop: &serde_json::Value) -> Option<String> {
    let title_arr = prop
        .get("title")
        .or_else(|| prop.get("rich_text"))?
        .as_array()?;
    Some(rich_text_array_to_string(title_arr))
}

/// Convert plain text to a minimal array of Notion paragraph blocks.
fn text_to_blocks(text: &str) -> Vec<serde_json::Value> {
    text.lines()
        .map(|line| {
            serde_json::json!({
                "object": "block",
                "type": "paragraph",
                "paragraph": {
                    "rich_text": [{ "type": "text", "text": { "content": line } }]
                }
            })
        })
        .collect()
}

fn check_status(resp: &reqwest::Response) -> Result<(), ProviderError> {
    let status = resp.status();
    if status.as_u16() == 401 {
        return Err(ProviderError::AuthRequired);
    }
    if !status.is_success() {
        return Err(ProviderError::Api(format!(
            "Notion API error: HTTP {}",
            status.as_u16()
        )));
    }
    Ok(())
}
