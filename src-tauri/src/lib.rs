mod github_auth;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn client_id() -> Option<String> {
    std::env::var("CLIENT_ID").ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Loads src-tauri/.env when running under `tauri dev`; missing file is not an error.
    dotenvy::dotenv().ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(github_auth::Auth::new())
        .invoke_handler(tauri::generate_handler![
            client_id,
            github_auth::start_device_auth,
            github_auth::complete_device_auth,
            github_auth::current_user,
            github_auth::sign_out,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
