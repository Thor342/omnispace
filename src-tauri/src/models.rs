use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Page {
    pub id: String,
    pub space_id: String,
    pub title: String,
    pub order_index: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub id: String,
    pub page_id: String,
    pub block_type: String, // "note" | "link" | "file" | "task" | "calendar"
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub content: String, // JSON string
    pub z_index: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub category_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: String,
    pub space_id: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppFile {
    pub id: String,
    pub space_id: String,
    pub name: String,
    pub original_path: String,
    pub stored_path: String,
    pub file_type: String, // "image", "pdf", "video", "other"
    pub size: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Link {
    pub id: String,
    pub space_id: String,
    pub title: String,
    pub url: String,
    pub link_type: String, // "youtube", "general"
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub space_id: String,
    pub title: String,
    pub completed: bool,
    pub order_index: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub order_index: i64,
    pub created_at: String,
    pub updated_at: String,
}
