use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

// ─── Graph data structures ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub provider_id: String,
    pub color: String,
}

#[derive(Debug, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    /// "backlink" | "semantic"
    pub kind: String,
    /// Similarity score (0–1) for semantic edges; 1.0 for backlinks.
    pub weight: f32,
}

#[derive(Debug, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

// ─── Command ──────────────────────────────────────────────────────────────────

/// Build the note graph data for the Graph View (S39).
///
/// Returns nodes (all visible notes) and two types of edges:
/// - `backlink`: explicit [[Note Title]] references in note content
/// - `semantic`: cosine similarity > threshold from the TF-IDF index
#[tauri::command]
pub async fn notes_get_graph(state: State<'_, AppState>) -> Result<GraphData, String> {
    let db = &state.db;

    // 1. Load all visible notes (id, title, provider_id, color, content_text)
    let notes: Vec<(String, String, String, String, String)> = db
        .with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, COALESCE(title, ''), provider_id, COALESCE(color, 'default'),
                            COALESCE(content_text, '')
                     FROM notes WHERE is_trashed = 0
                     ORDER BY updated_at DESC
                     LIMIT 500",
                )
                .map_err(|e| skeepy_core::StorageError::Database(e.to_string()))?;

            let rows: Result<Vec<_>, _> = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(|e| skeepy_core::StorageError::Database(e.to_string()))?
                .map(|r| r.map_err(|e| skeepy_core::StorageError::Database(e.to_string())))
                .collect();
            rows
        })
        .map_err(|e| e.to_string())?;

    // 2. Build title → id map for backlink resolution
    let title_to_id: std::collections::HashMap<String, String> = notes
        .iter()
        .filter(|(_, title, _, _, _)| !title.is_empty())
        .map(|(id, title, _, _, _)| (title.to_lowercase(), id.clone()))
        .collect();

    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut seen_pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    // 3. Extract backlinks from content: [[Note Title]]
    let backlink_re = backlink_pattern();
    for (note_id, _, _, _, content) in &notes {
        for cap in backlink_re.find_iter(content) {
            // Strip [[ and ]]
            let raw = &cap[2..cap.len() - 2];
            // Handle [[Target|Alias]] — use target part
            let target_name = raw.split('|').next().unwrap_or(raw).trim().to_lowercase();
            if let Some(target_id) = title_to_id.get(&target_name) {
                if target_id != note_id {
                    let pair = ordered_pair(note_id, target_id);
                    if seen_pairs.insert(pair) {
                        edges.push(GraphEdge {
                            source: note_id.clone(),
                            target: target_id.clone(),
                            kind: "backlink".to_string(),
                            weight: 1.0,
                        });
                    }
                }
            }
        }
    }

    // 4. Add semantic edges from note_embeddings cosine similarity.
    //    We only do this for the first 200 notes (O(n²) is fine for n≤200).
    //    Threshold: 0.30 cosine similarity.
    const SEMANTIC_THRESHOLD: f32 = 0.30;

    let embeddings: Vec<(String, Vec<f32>)> = db
        .with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT note_id, embedding FROM note_embeddings
                     WHERE model_version = 'tfidf-v1'
                     LIMIT 200",
                )
                .map_err(|e| skeepy_core::StorageError::Database(e.to_string()))?;

            let rows: Result<Vec<_>, _> = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let json: String = row.get(1)?;
                    Ok((id, json))
                })
                .map_err(|e| skeepy_core::StorageError::Database(e.to_string()))?
                .map(|r| r.map_err(|e| skeepy_core::StorageError::Database(e.to_string())))
                .collect();
            rows
        })
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|(id, json)| {
            let vec: Vec<f32> = serde_json::from_str(&json).ok()?;
            Some((id, vec))
        })
        .collect();

    let n = embeddings.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let sim = cosine_sim(&embeddings[i].1, &embeddings[j].1);
            if sim >= SEMANTIC_THRESHOLD {
                let pair = ordered_pair(&embeddings[i].0, &embeddings[j].0);
                if seen_pairs.insert(pair) {
                    edges.push(GraphEdge {
                        source: embeddings[i].0.clone(),
                        target: embeddings[j].0.clone(),
                        kind: "semantic".to_string(),
                        weight: sim,
                    });
                }
            }
        }
    }

    // 5. Build node list
    let nodes = notes
        .into_iter()
        .map(|(id, title, provider_id, color, _)| GraphNode {
            id,
            title: if title.is_empty() { "(sin título)".to_string() } else { title },
            provider_id,
            color,
        })
        .collect();

    Ok(GraphData { nodes, edges })
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() { return 0.0; }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn ordered_pair(a: &str, b: &str) -> (String, String) {
    if a < b { (a.to_string(), b.to_string()) } else { (b.to_string(), a.to_string()) }
}

/// Simple `[[...]]` pattern finder without the `regex` crate dependency.
struct BacklinkFinder<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> BacklinkFinder<'a> {
    fn find_iter(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }
}

impl<'a> Iterator for BacklinkFinder<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos + 4 <= self.text.len() {
            if let Some(start) = self.text[self.pos..].find("[[") {
                let abs_start = self.pos + start;
                if let Some(end) = self.text[abs_start + 2..].find("]]") {
                    let abs_end = abs_start + 2 + end + 2;
                    self.pos = abs_start + 2; // advance past [[
                    return Some(&self.text[abs_start..abs_end]);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        None
    }
}

fn backlink_pattern() -> BacklinkFinderFactory {
    BacklinkFinderFactory
}

struct BacklinkFinderFactory;
impl BacklinkFinderFactory {
    fn find_iter<'a>(&self, text: &'a str) -> BacklinkFinder<'a> {
        BacklinkFinder::find_iter(text)
    }
}
