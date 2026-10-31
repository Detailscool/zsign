#[tauri::command]
fn select_ipa() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("iOS App", &["ipa"])
        .pick_file()
        .map(|path| path.to_string_lossy().into_owned())
}

#[cfg(target_os = "macos")]
#[tauri::command]
async fn list_devices() -> Result<Vec<zsign_gui_core::devices::Device>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        use std::process::Command;
        let output_file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
        let mut command = Command::new("xcrun");
        // Respect an explicitly selected Xcode; otherwise find an installed Xcode if
        // Command Line Tools are selected and do not provide devicectl.
        if std::env::var_os("DEVELOPER_DIR").is_none() {
            let available = Command::new("xcrun")
                .args(["--find", "devicectl"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !available {
                let mut candidates: Vec<_> = std::fs::read_dir("/Applications")
                    .map_err(|e| e.to_string())?
                    .filter_map(Result::ok)
                    .map(|entry| entry.path().join("Contents/Developer"))
                    .filter(|path| path.join("usr/bin/devicectl").is_file())
                    .collect();
                candidates.sort();
                if let Some(path) = candidates.last() {
                    command.env("DEVELOPER_DIR", path);
                }
            }
        }
        let output = command
            .args(["devicectl", "list", "devices", "--json-output"])
            .arg(output_file.path())
            .output()
            .map_err(|e| format!("无法运行 devicectl：{e}"))?;
        if !output.status.success() {
            return Err("无法读取设备。请安装 Xcode 并检查开发工具配置。".into());
        }
        let json = std::fs::read_to_string(output_file.path()).map_err(|e| e.to_string())?;
        zsign_gui_core::devices::parse_devicectl_devices(&json)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
async fn list_devices() -> Result<Vec<zsign_gui_core::devices::Device>, String> {
    Err("此平台的设备发现功能正在开发".into())
}

fn main() {
    tauri::Builder::default()
        .manage(std::sync::Arc::new(auth::Authentication::default()))
        .invoke_handler(tauri::generate_handler![
            select_ipa,
            list_devices,
            auth::login,
            auth::submit_verification,
            auth::send_sms,
            auth::cancel_verification
        ])
        .run(tauri::generate_context!())
        .expect("could not start zsign desktop");
}
mod auth;
