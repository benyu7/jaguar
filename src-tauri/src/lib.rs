// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

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
        .invoke_handler(tauri::generate_handler![greet, client_id])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
