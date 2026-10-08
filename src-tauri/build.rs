fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "import_files",
            "authorize_directory",
            "list_grants",
            "revoke_grant",
            "search_files",
            "list_tags",
            "create_tag",
            "rename_tag",
            "set_file_tag",
            "file_details",
            "relink_file",
            "delete_index",
        ]),
    ))
    .expect("failed to generate desktop resources and command permissions");
}
