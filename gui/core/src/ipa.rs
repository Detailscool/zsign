use std::io::{Read, Seek};

#[derive(Debug)]
pub struct IpaInfo {
    pub bundle_id: String,
    pub name: String,
    pub executable: String,
    pub app_path: String,
}

/// Inspect an IPA without extracting it or modifying the input.
pub fn inspect_archive<R: Read + Seek>(reader: R) -> Result<IpaInfo, String> {
    let mut archive = zip::ZipArchive::new(reader).map_err(|_| "无法读取 IPA 压缩包")?;
    let mut infos = Vec::new();
    let mut names = std::collections::HashSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| "无法读取 IPA 条目")?;
        let name = entry.name();
        if entry.enclosed_name().is_none()
            || name.contains('\\')
            || name.contains(':')
            || !names.insert(name.to_string())
        {
            return Err("IPA 包含不安全或重复的文件路径".into());
        }
        let components: Vec<_> = name.split('/').filter(|part| !part.is_empty()).collect();
        if components.contains(&"..") {
            return Err("IPA 包含不安全的文件路径".into());
        }
        if components
            .iter()
            .any(|part| part.to_ascii_lowercase().ends_with(".appex"))
        {
            return Err("暂不支持包含 App 扩展的 IPA".into());
        }
        if components.first() == Some(&"Payload")
            && components
                .iter()
                .skip(2)
                .any(|part| part.to_ascii_lowercase().ends_with(".app"))
        {
            return Err("暂不支持嵌套 App".into());
        }
        if components.len() == 3
            && components[0] == "Payload"
            && components[1].ends_with(".app")
            && components[2] == "Info.plist"
        {
            infos.push((index, format!("Payload/{}", components[1])));
        }
    }
    if infos.len() != 1 {
        return Err("IPA 必须包含且只包含一个主 App".into());
    }
    let (index, app_path) = infos.pop().unwrap();
    let mut entry = archive.by_index(index).map_err(|_| "无法读取 Info.plist")?;
    if entry.size() > 4 * 1024 * 1024 {
        return Err("Info.plist 过大".into());
    }
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取 Info.plist")?;
    let value = plist::Value::from_reader(std::io::Cursor::new(bytes))
        .map_err(|_| "Info.plist 格式无效")?;
    let dictionary = value.as_dictionary().ok_or("Info.plist 格式无效")?;
    let text = |key: &str| {
        dictionary
            .get(key)
            .and_then(plist::Value::as_string)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let bundle_id = text("CFBundleIdentifier").ok_or("缺少 Bundle ID")?;
    let executable = text("CFBundleExecutable").ok_or("缺少主程序名称")?;
    if executable.contains('/')
        || executable.contains('\\')
        || executable == "."
        || executable == ".."
    {
        return Err("主程序名称无效".into());
    }
    if !names.contains(&format!("{app_path}/{executable}")) {
        return Err("IPA 缺少主程序文件".into());
    }
    let name = text("CFBundleDisplayName")
        .or_else(|| text("CFBundleName"))
        .unwrap_or_else(|| bundle_id.clone());
    Ok(IpaInfo {
        bundle_id,
        name,
        executable,
        app_path,
    })
}
