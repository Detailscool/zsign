pub fn validate_profile(
    profile: &plist::Dictionary,
    team: &str,
    bundle: &str,
    device: &str,
    cert: &[u8],
) -> Result<(), String> {
    let array_has_string = |key: &str, value: &str| {
        profile
            .get(key)
            .and_then(plist::Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item.as_string() == Some(value)))
    };
    if !array_has_string("TeamIdentifier", team) {
        return Err("描述文件的开发团队不匹配".into());
    }
    if !array_has_string("ProvisionedDevices", device) {
        return Err("描述文件未包含所选设备".into());
    }
    if !profile
        .get("DeveloperCertificates")
        .and_then(plist::Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_data() == Some(cert)))
    {
        return Err("描述文件未包含签名证书".into());
    }
    let expires: std::time::SystemTime = profile
        .get("ExpirationDate")
        .and_then(plist::Value::as_date)
        .ok_or("描述文件缺少有效期")?
        .into();
    if expires <= std::time::SystemTime::now() {
        return Err("描述文件已过期".into());
    }
    let app_id = profile
        .get("Entitlements")
        .and_then(plist::Value::as_dictionary)
        .and_then(|e| e.get("application-identifier"))
        .and_then(plist::Value::as_string)
        .ok_or("描述文件缺少 application-identifier")?;
    if app_id != format!("{team}.{bundle}") {
        return Err("描述文件的 Bundle ID 不匹配".into());
    }
    Ok(())
}
