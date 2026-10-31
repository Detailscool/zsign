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
async fn inspect_dropped_ipa(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let file = std::path::Path::new(&path);
        if !file
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ipa"))
        {
            return Err("请选择 IPA 文件".to_string());
        }
        ipa::inspect_ipa(file)?;
        Ok(path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn list_devices() -> Result<Vec<zsign_gui_core::devices::Device>, String> {
    tauri::async_runtime::spawn_blocking(devices::discover)
        .await
        .map_err(|e| e.to_string())?
}

fn configure_tls() {
    // Dependencies enable multiple rustls providers; choose one before any client is built.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn main() {
    configure_tls();
    tauri::Builder::default()
        .manage(std::sync::Arc::new(auth::Authentication::default()))
        .manage(std::sync::Arc::new(preparation::PlanStore::default()))
        .invoke_handler(tauri::generate_handler![
            select_ipa,
            inspect_dropped_ipa,
            list_devices,
            auth::login,
            auth::saved_login,
            auth::restore_session,
            auth::sign_out,
            auth::submit_verification,
            auth::list_teams,
            auth::send_sms,
            auth::cancel_verification,
            preparation::prepare_signing,
            execute::execute_signing
        ])
        .run(tauri::generate_context!())
        .expect("could not start zsign desktop");
}
mod auth;
mod certificates;
mod devices;
mod execute;
mod ipa;
mod package;
mod preparation;
mod secrets;
mod signer;

#[cfg(test)]
mod tests {
    #[test]
    fn tls_client_configuration_works_when_multiple_providers_are_enabled() {
        super::configure_tls();
        let _ = rustls::ClientConfig::builder();
    }
}
