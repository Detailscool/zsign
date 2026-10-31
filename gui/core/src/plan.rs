use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Certificate {
    pub serial: String,
    pub name: String,
}

#[derive(Clone)]
pub struct PlanInput {
    pub app_name: String,
    pub original_bundle_id: String,
    pub requested_bundle_id: Option<String>,
    pub original_registered: bool,
    pub requested_registered: bool,
    pub team_id: String,
    pub team_name: String,
    pub device_id: String,
    pub device_name: String,
    pub device_registered: bool,
    pub certificate: Option<Certificate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SigningPlan {
    pub app_name: String,
    pub original_bundle_id: String,
    pub target_bundle_id: String,
    pub team_id: String,
    pub team_name: String,
    pub device_id: String,
    pub device_name: String,
    pub register_device: bool,
    pub register_app_id: bool,
    pub create_certificate: bool,
    pub certificate: Option<Certificate>,
}

pub fn valid_bundle_id(id: &str) -> bool {
    id.len() <= 255
        && id.contains('.')
        && id.split('.').all(|part| {
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

pub fn build_plan(input: PlanInput) -> Result<SigningPlan, String> {
    if input.team_id.len() != 10
        || !input
            .team_id
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err("开发团队 ID 无效".into());
    }
    if !valid_bundle_id(&input.original_bundle_id) || input.device_id.is_empty() {
        return Err("应用 Bundle ID 或目标设备无效".into());
    }
    let target_bundle_id = input.requested_bundle_id.unwrap_or_else(|| {
        if input.original_registered {
            input.original_bundle_id.clone()
        } else {
            format!(
                "com.zsign.{}.{}",
                input.team_id.to_ascii_lowercase(),
                input.original_bundle_id
            )
        }
    });
    if !valid_bundle_id(&target_bundle_id) {
        return Err("建议的 Bundle ID 无效，请输入有效的标识".into());
    }
    let registered = if target_bundle_id == input.original_bundle_id {
        input.original_registered
    } else {
        input.requested_registered
    };
    Ok(SigningPlan {
        app_name: input.app_name,
        original_bundle_id: input.original_bundle_id,
        target_bundle_id,
        team_id: input.team_id,
        team_name: input.team_name,
        device_id: input.device_id,
        device_name: input.device_name,
        register_device: !input.device_registered,
        register_app_id: !registered,
        create_certificate: input.certificate.is_none(),
        certificate: input.certificate,
    })
}
