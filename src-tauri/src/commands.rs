use crate::{
    access::Snapshot,
    error::{AppError, Result},
    model::{FileRecord, Grant, ImportReport, Tag},
    repository::Repository,
    AppState,
};
use std::sync::MutexGuard;
use tauri::State;
fn lock<'a>(state: &'a State<'_, AppState>) -> Result<MutexGuard<'a, Repository>> {
    state
        .0
        .lock()
        .map_err(|_| AppError::new("STATE", "应用状态异常，请重启程序。"))
}
#[tauri::command]
pub async fn import_files(state: State<'_, AppState>) -> Result<ImportReport> {
    let files = rfd::AsyncFileDialog::new()
        .set_title("选择文件：仅登记路径和元数据，并授权后续按需检查")
        .pick_files()
        .await;
    let mut report = ImportReport {
        imported: 0,
        errors: vec![],
    };
    if let Some(files) = files {
        if files.len() > 500 {
            return Err(AppError::new("LIMIT", "每次最多登记 500 个文件。"));
        }
        let mut db = lock(&state)?;
        for (index, file) in files.iter().enumerate() {
            match Snapshot::open_selected(file.path(), false).and_then(|s| db.register(&s)) {
                Ok(_) => report.imported += 1,
                Err(e) => report
                    .errors
                    .push(format!("第 {} 项：{}", index + 1, e.message)),
            }
        }
    }
    Ok(report)
}
#[tauri::command]
pub async fn authorize_directory(state: State<'_, AppState>) -> Result<bool> {
    if let Some(folder) = rfd::AsyncFileDialog::new()
        .set_title("授权此目录内已登记文件的按需读取（不扫描目录）")
        .pick_folder()
        .await
    {
        let s = Snapshot::open_selected(folder.path(), true)?;
        lock(&state)?.grant_directory(&s)?;
        return Ok(true);
    }
    Ok(false)
}
#[tauri::command]
pub fn list_grants(state: State<'_, AppState>) -> Result<Vec<Grant>> {
    lock(&state)?.grants()
}
#[tauri::command]
pub fn revoke_grant(state: State<'_, AppState>, id: i64) -> Result<()> {
    lock(&state)?.revoke(id)
}
#[tauri::command]
pub fn search_files(
    state: State<'_, AppState>,
    query: String,
    tag_id: Option<i64>,
    extension: String,
) -> Result<Vec<FileRecord>> {
    lock(&state)?.search(&query, tag_id, &extension)
}
#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<Tag>> {
    lock(&state)?.tags()
}
#[tauri::command]
pub fn create_tag(state: State<'_, AppState>, name: String) -> Result<()> {
    lock(&state)?.create_tag(&name)
}
#[tauri::command]
pub fn rename_tag(state: State<'_, AppState>, id: i64, name: String) -> Result<()> {
    lock(&state)?.rename_tag(id, &name)
}
#[tauri::command]
pub fn set_file_tag(
    state: State<'_, AppState>,
    file_id: i64,
    tag_id: i64,
    attached: bool,
) -> Result<()> {
    lock(&state)?.set_tag(file_id, tag_id, attached)
}
#[tauri::command]
pub fn file_details(state: State<'_, AppState>, id: i64) -> Result<FileRecord> {
    lock(&state)?.refresh(id)
}
#[tauri::command]
pub async fn relink_file(state: State<'_, AppState>, id: i64) -> Result<bool> {
    // Validate the record before presenting the external-file selection dialog.
    lock(&state)?.file(id)?;
    if let Some(file) = rfd::AsyncFileDialog::new()
        .set_title("重新关联：请选择同一卷内移动后的原文件（身份必须匹配）")
        .pick_file()
        .await
    {
        let s = Snapshot::open_selected(file.path(), false)?;
        lock(&state)?.relink(id, &s)?;
        return Ok(true);
    }
    Ok(false)
}
#[tauri::command]
pub fn delete_index(state: State<'_, AppState>, id: i64) -> Result<()> {
    lock(&state)?.delete_index(id)
}
