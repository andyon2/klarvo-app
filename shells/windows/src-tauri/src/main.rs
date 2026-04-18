#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let specta_builder = klarvo_windows_shell::specta_builder();

    tauri::Builder::default()
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
