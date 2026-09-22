mod github_auth;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Loads src-tauri/.env when running under `tauri dev`; missing file is not an error.
    dotenvy::dotenv().ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(github_auth::Auth::new())
        .invoke_handler(tauri::generate_handler![
            github_auth::start_device_auth,
            github_auth::complete_device_auth,
            github_auth::current_user,
            github_auth::sign_out,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
