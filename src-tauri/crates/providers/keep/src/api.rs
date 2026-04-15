use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use skeepy_core::ProviderError;

const NOTES_BASE: &str = "https://notes.googleapis.com/v1";
const PAGE_SIZE: u32 = 100;

// ─── API response types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeepNote {
    pub name: String,
    pub create_time: String,
    pub update_time: String,
    #[allow(dead_code)]
    pub trash_time: Option<String>,
    #[serde(default)]
    pub trashed: bool,
    pub title: Option<String>,
    pub body: Option<NoteBody>,
    #[serde(default)]
    pub labels: Vec<NoteLabel>,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub starred: bool,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteBody {
    pub text: Option<TextContent>,
    pub list: Option<ListContent>,
}

#[derive(Debug, Deserialize)]
pub struct TextContent {
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListContent {
    pub list_items: Vec<ListItem>,
}

#[derive(Debug, Deserialize)]
pub struct ListItem {
    pub text: Option<TextContent>,
    #[serde(default)]
    pub checked: bool,
    #[serde(default, rename = "childListItems")]
    pub child_items: Vec<ListItem>,
}

#[derive(Debug, Deserialize)]
pub struct NoteLabel {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListNotesResponse {
    #[serde(default)]
    notes: Vec<KeepNote>,
    next_page_token: Option<String>,
}

// ─── Write request types ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub body: CreateNoteBody,
}

#[derive(Debug, Serialize)]
pub struct CreateNoteBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<CreateTextBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<CreateListBody>,
}

#[derive(Debug, Serialize)]
pub struct CreateTextBody {
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateListBody {
    pub list_items: Vec<CreateListItem>,
}

#[derive(Debug, Serialize)]
pub struct CreateListItem {
    pub text: CreateTextBody,
    pub checked: bool,
}

// ─── API client ───────────────────────────────────────────────────────────────

/// HTTP client for the Google Keep API.
/// Rate-limiting is the caller's responsibility — see `KeepProvider`.
pub struct KeepApiClient {
    http: reqwest::Client,
}

impl KeepApiClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("SkeepyNotes/0.1")
            .build()
            .expect("failed to build reqwest client");
        Self { http }
    }

    /// Fetch all notes, handling pagination automatically.
    /// `since` is an optional RFC-3339 timestamp for incremental sync.
    pub async fn list_all_notes(
        &self,
        access_token: &str,
        since: Option<&DateTime<Utc>>,
    ) -> Result<Vec<KeepNote>, ProviderError> {
        let filter = since.map(|t| format!("updateTime>\"{}\"", t.to_rfc3339()));

        let mut all = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self
                .http
                .get(format!("{NOTES_BASE}/notes"))
                .bearer_auth(access_token)
                .query(&[("pageSize", PAGE_SIZE.to_string())]);

            if let Some(ref f) = filter {
                req = req.query(&[("filter", f.as_str())]);
            }
            if let Some(ref token) = page_token {
                req = req.query(&[("pageToken", token.as_str())]);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| ProviderError::Api(format!("Keep list request failed: {e}")))?;

            check_status(&resp)?;

            let body: ListNotesResponse = resp
                .json()
                .await
                .map_err(|e| ProviderError::Api(format!("Keep list parse error: {e}")))?;

            debug!(count = body.notes.len(), has_next = body.next_page_token.is_some(), "Keep list page");

            all.extend(body.notes);

            match body.next_page_token {
                Some(token) => page_token = Some(token),
                None => break,
            }
        }

        Ok(all)
    }

    /// Fetch a single note by its resource name (e.g. `notes/AAAAAJxxxxxxx`).
    pub async fn get_note(
        &self,
        access_token: &str,
        name: &str,
    ) -> Result<KeepNote, ProviderError> {
        let resp = self
            .http
            .get(format!("{NOTES_BASE}/{name}"))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Keep get request failed: {e}")))?;

        check_status(&resp)?;

        resp.json::<KeepNote>()
            .await
            .map_err(|e| ProviderError::Api(format!("Keep get parse error: {e}")))
    }

    /// Create a new note in Google Keep.
    pub async fn create_note(
        &self,
        access_token: &str,
        req: CreateNoteRequest,
    ) -> Result<KeepNote, ProviderError> {
        let resp = self
            .http
            .post(format!("{NOTES_BASE}/notes"))
            .bearer_auth(access_token)
            .json(&req)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Keep create request failed: {e}")))?;

        check_status(&resp)?;

        resp.json::<KeepNote>()
            .await
            .map_err(|e| ProviderError::Api(format!("Keep create parse error: {e}")))
    }

    /// Delete a note by its resource name (e.g. `notes/AAAAAJxxxxxxx`).
    pub async fn delete_note(
        &self,
        access_token: &str,
        name: &str,
    ) -> Result<(), ProviderError> {
        let resp = self
            .http
            .delete(format!("{NOTES_BASE}/{name}"))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Keep delete request failed: {e}")))?;

        check_status(&resp)?;
        Ok(())
    }
}

fn check_status(resp: &reqwest::Response) -> Result<(), ProviderError> {
    let status = resp.status();
    if status.as_u16() == 401 {
        return Err(ProviderError::AuthRequired);
    }
    if !status.is_success() {
        return Err(ProviderError::Api(format!("Keep API error: HTTP {}", status.as_u16())));
    }
    Ok(())
}
