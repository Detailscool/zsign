use std::{ffi::OsString, path::Path, process::Command};

pub fn cli_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let candidate = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join(if cfg!(windows) { "zsign.exe" } else { "zsign" });
    if candidate.is_file() {
        return Ok(candidate);
    }
    #[cfg(debug_assertions)]
    {
        let candidate = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bin/zsign");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("未找到 zsign 签名程序，请先构建或安装完整应用".into())
}

pub fn run_cli(executable: &Path, args: &[OsString], working_dir: &Path) -> Result<(), String> {
    let output = Command::new(executable)
        .args(args)
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("无法启动 zsign：{e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail: String = format!("{stderr}\n{stdout}").chars().take(4000).collect();
        return Err(format!("签名失败：{}", detail.trim()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    #[test]
    fn a_signing_process_failure_is_reported() {
        let error = super::run_cli(
            std::path::Path::new("/usr/bin/false"),
            &[],
            std::path::Path::new("/tmp"),
        )
        .unwrap_err();
        assert!(error.contains("签名失败"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "requires ZSIGN_TEST_BINARY built for this host"]
    fn signs_a_minimal_app_with_the_real_cli() {
        use std::{fs, os::unix::fs::PermissionsExt, process::Command};
        let cli = std::env::var_os("ZSIGN_TEST_BINARY").expect("set ZSIGN_TEST_BINARY");
        let work = tempfile::tempdir().unwrap();
        let app = work.path().join("Example.app");
        fs::create_dir(&app).unwrap();
        fs::write(app.join("Info.plist"), br#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.example.app</string><key>CFBundleExecutable</key><string>Example</string><key>CFBundleName</key><string>Example</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#).unwrap();
        let executable = app.join("Example");
        fs::write(
            &executable,
            include_bytes!("../tests/fixtures/signed-basic.macho"),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        let modes = crate::package::capture_permissions(&app).unwrap();
        super::run_cli(
            std::path::Path::new(&cli),
            &["-a".into(), "-f".into(), app.as_os_str().into()],
            work.path(),
        )
        .unwrap();
        crate::package::restore_permissions(&app, &modes).unwrap();
        assert!(Command::new("codesign")
            .args(["--verify", "--deep"])
            .arg(&app)
            .status()
            .unwrap()
            .success());
        assert_eq!(
            fs::metadata(executable).unwrap().permissions().mode() & 0o111,
            0o111
        );
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "requires ZSIGN_TEST_BINARY built for this host"]
    fn signs_and_repackages_an_ipa_with_a_valid_bundle_signature() {
        use std::{fs, os::unix::fs::PermissionsExt, process::Command};
        let cli = std::env::var_os("ZSIGN_TEST_BINARY").expect("set ZSIGN_TEST_BINARY");
        let work = tempfile::tempdir().unwrap();
        let original = work.path().join("original");
        let app = original.join("Payload/Example.app");
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("Info.plist"), br#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.example.app</string><key>CFBundleExecutable</key><string>Example</string><key>CFBundleName</key><string>Example</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#).unwrap();
        let executable = app.join("Example");
        fs::write(
            &executable,
            include_bytes!("../tests/fixtures/signed-basic.macho"),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        let input = work.path().join("input.ipa");
        crate::package::archive(&original, &input).unwrap();
        let extracted = work.path().join("extracted");
        crate::package::extract(&input, &extracted).unwrap();
        let signed_app = extracted.join("Payload/Example.app");
        let modes = crate::package::capture_permissions(&signed_app).unwrap();
        super::run_cli(
            std::path::Path::new(&cli),
            &["-a".into(), "-f".into(), signed_app.as_os_str().into()],
            work.path(),
        )
        .unwrap();
        crate::package::restore_permissions(&signed_app, &modes).unwrap();
        let output = work.path().join("output.ipa");
        crate::package::archive(&extracted, &output).unwrap();
        let result = work.path().join("result");
        crate::package::extract(&output, &result).unwrap();
        let result_app = result.join("Payload/Example.app");
        assert!(Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(&result_app)
            .status()
            .unwrap()
            .success());
        assert_eq!(
            fs::metadata(result_app.join("Example"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0o111
        );
    }
}
