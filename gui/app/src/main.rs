#[tauri::command]
fn select_ipa() -> Result<Option<String>, String> {
    let path = rfd::FileDialog::new()
        .add_filter("iOS App", &["ipa"])
        .pick_file();
    let Some(path) = path else {
        return Ok(None);
    };
    ipa::inspect_ipa(&path)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn list_devices() -> Result<Vec<zsign_gui_core::devices::Device>, String> {
    tauri::async_runtime::spawn_blocking(devices::discover)
        .await
        .map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .manage(std::sync::Arc::new(auth::Authentication::default()))
        .manage(std::sync::Arc::new(preparation::PlanStore::default()))
        .invoke_handler(tauri::generate_handler![
            select_ipa,
            list_devices,
            auth::login,
            auth::restore_session,
            auth::sign_out,
            auth::submit_verification,
            auth::list_teams,
            auth::send_sms,
            auth::cancel_verification,
            preparation::prepare_signing
        ])
        .run(tauri::generate_context!())
        .expect("could not start zsign desktop");
}
mod auth;
mod certificates;
mod devices;
mod ipa;
mod package;
mod preparation;
mod secrets;
