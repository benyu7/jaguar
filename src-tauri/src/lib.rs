mod gh;
mod pull_requests;
mod session;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(session::Session::new())
        .invoke_handler(tauri::generate_handler![
            session::current_user,
            pull_requests::list_my_pull_requests,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
