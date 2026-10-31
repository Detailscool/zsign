#[tauri::command]
fn select_ipa() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("iOS App", &["ipa"])
        .pick_file()
        .map(|path| path.to_string_lossy().into_owned())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![select_ipa])
        .run(tauri::generate_context!())
        .expect("could not start zsign desktop");
}
