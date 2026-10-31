use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Device {
    pub id: String,
    pub udid: String,
    pub name: String,
}

/// Extract connected physical iOS devices from `devicectl list devices` JSON.
pub fn parse_devicectl_devices(output: &str) -> Result<Vec<Device>, String> {
    let value: Value = serde_json::from_str(output).map_err(|e| e.to_string())?;
    let devices = value.pointer("/result/devices").and_then(Value::as_array)
        .ok_or("devicectl response does not contain a device list")?;
    let mut result = Vec::new();
    for device in devices {
        let properties = &device["properties"];
        let hardware = &properties["hardware"];
        if hardware["reality"].as_str() != Some("physical")
            || hardware["platform"].as_str() != Some("iOS")
            || properties["connection"]["state"].as_str() != Some("connected")
        { continue; }
        if let (Some(id), Some(udid), Some(name)) = (
            device["identifier"].as_str(), hardware["udid"].as_str(),
            properties["state"]["name"].as_str(),
        ) {
            result.push(Device { id: id.into(), udid: udid.into(), name: name.into() });
        }
    }
    Ok(result)
}
