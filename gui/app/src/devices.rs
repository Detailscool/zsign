use zsign_gui_core::devices::Device;

#[cfg(target_os = "macos")]
pub fn devicectl() -> Result<std::process::Command, String> {
    use std::process::Command;
    let mut command = Command::new("xcrun");
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
    command.arg("devicectl");
    Ok(command)
}

#[cfg(target_os = "macos")]
pub fn discover() -> Result<Vec<Device>, String> {
    let output_file = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    let output = devicectl()?
        .args(["list", "devices", "--json-output"])
        .arg(output_file.path())
        .output()
        .map_err(|e| format!("无法运行 devicectl：{e}"))?;
    if !output.status.success() {
        return Err("无法读取设备。请安装 Xcode 并检查开发工具配置。".into());
    }
    let json = std::fs::read_to_string(output_file.path()).map_err(|e| e.to_string())?;
    zsign_gui_core::devices::parse_devicectl_devices(&json)
}

#[cfg(not(target_os = "macos"))]
pub fn discover() -> Result<Vec<Device>, String> {
    Err("此平台的设备发现功能正在开发".into())
}
