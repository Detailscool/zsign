use std::{fs, io, path::Path};
use zip::write::SimpleFileOptions;

pub type PermissionSnapshot = std::collections::HashMap<std::path::PathBuf, u32>;

pub fn capture_permissions(root: &Path) -> Result<PermissionSnapshot, String> {
    let mut modes = PermissionSnapshot::new();
    #[cfg(unix)]
    for entry in walkdir::WalkDir::new(root).min_depth(1).follow_links(false) {
        use std::os::unix::fs::PermissionsExt;
        let entry = entry.map_err(|e| e.to_string())?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        if metadata.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| "应用路径无效")?;
            modes.insert(
                relative.to_path_buf(),
                metadata.permissions().mode() & 0o777,
            );
        }
    }
    #[cfg(not(unix))]
    let _ = root;
    Ok(modes)
}

pub fn restore_permissions(root: &Path, modes: &PermissionSnapshot) -> Result<(), String> {
    #[cfg(unix)]
    for (relative, mode) in modes {
        use std::os::unix::fs::PermissionsExt;
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("签名后应用文件类型发生变化".into());
        }
        fs::set_permissions(path, fs::Permissions::from_mode(*mode)).map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    let _ = (root, modes);
    Ok(())
}

pub fn extract(ipa: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| format!("无法创建临时目录：{e}"))?;
    let file = fs::File::open(ipa).map_err(|e| format!("无法打开 IPA：{e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| "无法读取 IPA 压缩包")?;
    zip.extract(destination)
        .map_err(|e| format!("无法安全解压 IPA：{e}"))
}

pub fn archive(root: &Path, output: &Path) -> Result<(), String> {
    let file = fs::File::create(output).map_err(|e| format!("无法创建 IPA：{e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    for entry in walkdir::WalkDir::new(root).min_depth(1).follow_links(false) {
        let entry = entry.map_err(|e| format!("无法读取应用文件：{e}"))?;
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| "应用路径无效")?;
        let name = relative
            .components()
            .map(|part| part.as_os_str().to_str().ok_or("应用文件名无效"))
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o777
        };
        #[cfg(not(unix))]
        let mode = if metadata.is_dir() { 0o755 } else { 0o644 };
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(mode);
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(entry.path()).map_err(|e| e.to_string())?;
            let target = target.to_str().ok_or("符号链接目标无效")?;
            zip.add_symlink(name, target, options)
                .map_err(|e| e.to_string())?;
        } else if metadata.is_dir() {
            zip.add_directory(format!("{name}/"), options)
                .map_err(|e| e.to_string())?;
        } else if metadata.is_file() {
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            let mut source = fs::File::open(entry.path()).map_err(|e| e.to_string())?;
            io::copy(&mut source, &mut zip).map_err(|e| e.to_string())?;
        } else {
            return Err("应用包含不支持的文件类型".into());
        }
    }
    zip.finish().map_err(|e| format!("无法完成 IPA：{e}"))?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
    };

    #[test]
    fn ipa_roundtrip_preserves_executable_permissions_and_framework_symlinks() {
        let workspace = tempfile::tempdir().unwrap();
        let input = workspace.path().join("input");
        let app = input.join("Payload/Example.app");
        fs::create_dir_all(app.join("Frameworks/F.framework/Versions/A")).unwrap();
        let executable = app.join("Example");
        fs::write(&executable, b"test executable").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        symlink("A", app.join("Frameworks/F.framework/Versions/Current")).unwrap();
        let ipa = workspace.path().join("output.ipa");
        super::archive(&input, &ipa).unwrap();
        let output = workspace.path().join("unpacked");
        super::extract(&ipa, &output).unwrap();
        let extracted = output.join("Payload/Example.app/Example");
        assert_eq!(fs::read(extracted.clone()).unwrap(), b"test executable");
        assert_eq!(
            fs::metadata(extracted).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::read_link(
                output.join("Payload/Example.app/Frameworks/F.framework/Versions/Current")
            )
            .unwrap(),
            std::path::Path::new("A")
        );
    }

    #[test]
    fn executable_mode_survives_a_signer_replacing_the_file() {
        let workspace = tempfile::tempdir().unwrap();
        let executable = workspace.path().join("Example");
        fs::write(&executable, b"old executable").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        let modes = super::capture_permissions(workspace.path()).unwrap();
        fs::remove_file(&executable).unwrap();
        fs::write(&executable, b"new executable").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o644)).unwrap();
        super::restore_permissions(workspace.path(), &modes).unwrap();
        assert_eq!(
            fs::metadata(executable).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
}
