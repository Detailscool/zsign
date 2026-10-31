use plume_core::MachOExt;
use std::{io::Read, path::Path};
use zsign_gui_core::ipa::{inspect_archive, validate_entitlements, IpaInfo};

pub fn inspect_ipa(path: &Path) -> Result<IpaInfo, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("无法打开 IPA：{e}"))?;
    let info = inspect_archive(file)?;
    let file = std::fs::File::open(path).map_err(|e| format!("无法打开 IPA：{e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| "无法读取 IPA 压缩包")?;
    let executable_path = format!("{}/{}", info.app_path, info.executable);
    let entry = zip
        .by_name(&executable_path)
        .map_err(|_| "无法读取主程序")?;
    const MAX_EXECUTABLE_SIZE: u64 = 512 * 1024 * 1024;
    if entry.size() > MAX_EXECUTABLE_SIZE {
        return Err("主程序过大，无法检查签名".into());
    }
    let mut executable = Vec::new();
    entry
        .take(MAX_EXECUTABLE_SIZE + 1)
        .read_to_end(&mut executable)
        .map_err(|_| "无法读取主程序")?;
    if executable.len() as u64 > MAX_EXECUTABLE_SIZE {
        return Err("主程序过大，无法检查签名".into());
    }
    let macho =
        apple_codesign::MachFile::parse(&executable).map_err(|_| "主程序不是有效的 Mach-O")?;
    let mut slices = 0;
    for slice in macho.iter_macho() {
        slices += 1;
        if slice.macho.header.filetype != 2 {
            // MH_EXECUTE
            return Err("主程序不是可执行 Mach-O".into());
        }
        let entitlements = slice
            .embedded_entitlements()
            .map_err(|_| "无法读取原签名 entitlement")?
            .ok_or("主程序缺少可读取的签名 entitlement，无法确认支持的能力")?;
        validate_entitlements(&entitlements, &info.bundle_id)?;
    }
    if slices == 0 {
        return Err("主程序没有可读取的架构".into());
    }
    Ok(info)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    fn archive_with(executable: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(file.as_file_mut());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("Payload/Example.app/Info.plist", options)
            .unwrap();
        zip.write_all(br#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.example.app</string><key>CFBundleExecutable</key><string>Example</string></dict></plist>"#).unwrap();
        zip.start_file("Payload/Example.app/Example", options)
            .unwrap();
        zip.write_all(executable).unwrap();
        zip.finish().unwrap();
        file
    }

    #[test]
    fn rejects_an_unsigned_main_executable() {
        let file = archive_with(include_bytes!(
            "../tests/fixtures/unsigned-executable.macho"
        ));
        let error = super::inspect_ipa(file.path()).unwrap_err();
        assert!(error.contains("entitlement"), "{error}");
    }

    #[test]
    fn accepts_a_signed_app_with_only_basic_entitlements() {
        let file = archive_with(include_bytes!("../tests/fixtures/signed-basic.macho"));
        let info = super::inspect_ipa(file.path()).unwrap();
        assert_eq!(info.bundle_id, "com.example.app");
    }

    #[test]
    fn rejects_a_signed_library_as_the_main_executable() {
        let file = archive_with(include_bytes!("../tests/fixtures/signed-library.macho"));
        let error = super::inspect_ipa(file.path()).unwrap_err();
        assert!(error.contains("可执行"), "{error}");
    }

    #[test]
    fn rejects_a_real_signature_with_an_unsupported_capability() {
        let file = archive_with(include_bytes!("../tests/fixtures/signed-push.macho"));
        let error = super::inspect_ipa(file.path()).unwrap_err();
        assert!(error.contains("aps-environment"), "{error}");
    }
}
