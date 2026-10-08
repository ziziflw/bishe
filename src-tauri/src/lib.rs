mod access;
mod commands;
mod error;
mod model;
mod repository;
#[cfg(test)]
mod tests;
use repository::Repository;
use std::sync::Mutex;
use tauri::Manager;
pub struct AppState(Mutex<Repository>);
pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let executable = std::env::current_exe()?;
            let directory = executable
                .parent()
                .ok_or("No executable directory")?
                .join("PersonalFileTags-data");
            std::fs::create_dir_all(&directory)?;
            let repository = Repository::open(&directory.join("personal-file-tags.sqlite3"))?;
            app.manage(AppState(Mutex::new(repository)));
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Personal File Tags")
            .inner_size(1280.0, 820.0)
            .min_inner_size(960.0, 640.0)
            .disable_drag_drop_handler()
            .data_directory(directory.join("webview"))
            .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_files,
            commands::authorize_directory,
            commands::list_grants,
            commands::revoke_grant,
            commands::search_files,
            commands::list_tags,
            commands::create_tag,
            commands::rename_tag,
            commands::set_file_tag,
            commands::file_details,
            commands::relink_file,
            commands::delete_index
        ])
        .run(tauri::generate_context!());
    if result.is_err() {
        rfd::MessageDialog::new().set_title("Personal File Tags 启动失败")
            .set_description("无法初始化应用或数据库。请检查应用数据目录权限与磁盘空间；如数据库异常，请保留原数据库并恢复可靠备份。程序不会覆盖重建已有数据库。")
            .set_level(rfd::MessageLevel::Error).show();
    }
}
