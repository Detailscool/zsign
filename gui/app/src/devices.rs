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
    use std::process::Command;
    let output = Command::new("idevice_id")
        .arg("-l")
        .output()
        .map_err(|_| "找不到 idevice_id，请安装 libimobiledevice 并启动 usbmuxd")?;
    if !output.status.success() {
        return Err("无法读取设备，请检查 usbmuxd 和设备连接".into());
    }
    let text = String::from_utf8(output.stdout).map_err(|_| "设备列表格式无效")?;
    let mut devices = Vec::new();
    for udid in zsign_gui_core::devices::parse_usbmux_ids(&text) {
        let info = Command::new("ideviceinfo")
            .args(["-u", &udid, "-k", "DeviceName"])
            .output()
            .map_err(|_| "找不到 ideviceinfo，请安装 libimobiledevice")?;
        if !info.status.success() {
            continue;
        }
        let name = String::from_utf8_lossy(&info.stdout).trim().to_string();
        devices.push(Device {
            id: udid.clone(),
            udid,
            name: if name.is_empty() {
                "iOS 设备".into()
            } else {
                name
            },
        });
    }
    Ok(devices)
}
