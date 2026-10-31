use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Device {
    pub id: String,
    pub udid: String,
    pub name: String,
}

pub fn parse_usbmux_ids(output: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for id in output.lines().map(str::trim) {
        let valid = (id.len() == 40 && id.bytes().all(|b| b.is_ascii_hexdigit()))
            || (id.len() == 25
                && id.as_bytes().get(8) == Some(&b'-')
                && id
                    .bytes()
                    .enumerate()
                    .all(|(i, b)| i == 8 || b.is_ascii_hexdigit()));
        if valid
            && !ids
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(id))
        {
            ids.push(id.to_string());
        }
    }
    ids
}

/// Extract connected physical iOS devices from `devicectl list devices` JSON.
pub fn parse_devicectl_devices(output: &str) -> Result<Vec<Device>, String> {
    let value: Value = serde_json::from_str(output).map_err(|e| e.to_string())?;
    let devices = value
        .pointer("/result/devices")
        .and_then(Value::as_array)
        .ok_or("devicectl response does not contain a device list")?;
    let mut result = Vec::new();
    for device in devices {
        let properties = &device["properties"];
        let hardware = &properties["hardware"];
        if hardware["reality"].as_str() != Some("physical")
            || hardware["platform"].as_str() != Some("iOS")
            || properties["connection"]["state"].as_str() != Some("connected")
        {
            continue;
        }
        if let (Some(id), Some(udid), Some(name)) = (
            device["identifier"].as_str(),
            hardware["udid"].as_str(),
            properties["state"]["name"].as_str(),
        ) {
            result.push(Device {
                id: id.into(),
                udid: udid.into(),
                name: name.into(),
            });
        }
    }
    Ok(result)
}
