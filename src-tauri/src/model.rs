use serde::Serialize;
#[derive(Debug, Serialize, Clone)]
pub struct FileRecord {
    pub id: i64,
    pub identity: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: i64,
    pub modified_at: i64,
    pub registered_at: i64,
    pub status: String,
    pub tags: Vec<Tag>,
}
#[derive(Debug, Serialize, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}
#[derive(Debug, Serialize, Clone)]
pub struct Grant {
    pub id: i64,
    pub kind: String,
    pub path: String,
    pub identity: String,
    pub active: bool,
}
#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub imported: usize,
    pub errors: Vec<String>,
}
