use crate::{
    auth::{self, Authentication},
    certificates, devices, ipa,
    secrets::SystemSecretStore,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::State;
use zsign_gui_core::plan::{build_plan, Certificate, PlanInput, SigningPlan};

#[derive(Default)]
pub struct PlanStore(pub Mutex<Option<StoredPlan>>);

pub struct StoredPlan {
    pub token: String,
    pub plan: SigningPlan,
    pub ipa_path: PathBuf,
    pub digest: Vec<u8>,
    pub device_udid: String,
    pub account_generation: u64,
}

impl PlanStore {
    pub fn consume(&self, token: &str, generation: u64) -> Result<StoredPlan, String> {
        let mut guard = self.0.lock().map_err(|_| "签名计划不可用")?;
        let plan = guard.take().ok_or("签名预览已失效，请重新检查")?;
        if plan.token != token || plan.account_generation != generation {
            return Err("签名预览已失效，请重新检查".into());
        }
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confirmation_is_bound_to_one_plan_and_cannot_be_replayed() {
        let plan = zsign_gui_core::plan::build_plan(zsign_gui_core::plan::PlanInput {
            app_name: "App".into(),
            original_bundle_id: "com.example.app".into(),
            requested_bundle_id: None,
            original_registered: true,
            requested_registered: false,
            team_id: "TEAM123456".into(),
            team_name: "Team".into(),
            device_id: "device".into(),
            device_name: "Phone".into(),
            device_registered: true,
            certificate: None,
        })
        .unwrap();
        let store = PlanStore(Mutex::new(Some(StoredPlan {
            token: "token".into(),
            plan,
            ipa_path: PathBuf::from("app.ipa"),
            digest: vec![],
            device_udid: "device".into(),
            account_generation: 1,
        })));
        assert!(store.consume("token", 1).is_ok());
        assert!(store.consume("token", 1).is_err());
    }
}

#[derive(Serialize)]
pub struct Preview {
    token: String,
    plan: SigningPlan,
}

pub fn hash_file(path: &std::path::Path) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("无法打开 IPA：{e}"))?;
    let mut digest = Sha256::new();
    let mut buf = [0u8; 128 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|_| "无法读取 IPA")?;
        if n == 0 {
            break;
        }
        digest.update(&buf[..n]);
    }
    Ok(digest.finalize().to_vec())
}

#[tauri::command]
pub async fn prepare_signing(
    auth_state: State<'_, Arc<Authentication>>,
    plan_state: State<'_, Arc<PlanStore>>,
    ipa_path: String,
    team_id: String,
    device_id: String,
    bundle_id: Option<String>,
) -> Result<Preview, String> {
    *plan_state.0.lock().map_err(|_| "签名计划不可用")? = None;
    let generation = auth_state.generation();
    let path = PathBuf::from(ipa_path);
    let inspect_path = path.clone();
    let (info, digest, connected) = tauri::async_runtime::spawn_blocking(move || {
        let before = hash_file(&inspect_path)?;
        let info = ipa::inspect_ipa(&inspect_path)?;
        if hash_file(&inspect_path)? != before {
            return Err("IPA 在检查期间发生变化，请重新选择".to_string());
        }
        Ok::<_, String>((info, before, devices::discover()?))
    })
    .await
    .map_err(|e| e.to_string())??;
    let device = connected
        .into_iter()
        .find(|device| device.id == device_id)
        .ok_or("所选设备不再连接，请刷新设备列表")?;
    let session = auth::developer_session(&auth_state).await?;
    let team = session
        .qh_list_teams()
        .await
        .map_err(|e| e.to_string())?
        .teams
        .into_iter()
        .find(|team| team.team_id == team_id && team.status.eq_ignore_ascii_case("active"))
        .ok_or("所选开发团队不可用，请刷新团队列表")?;
    let device_registered = match session
        .qh_get_device(&team_id, &device.udid)
        .await
        .map_err(|e| e.to_string())?
    {
        None => false,
        Some(device)
            if device.status.eq_ignore_ascii_case("enabled")
                || device.status.eq_ignore_ascii_case("active") =>
        {
            true
        }
        Some(_) => return Err("该设备在开发者账户中未启用，请先检查账户中的设备状态".into()),
    };
    let app_ids = session
        .qh_list_app_ids(&team_id)
        .await
        .map_err(|e| e.to_string())?
        .app_ids;
    let original_registered = app_ids.iter().any(|item| item.identifier == info.bundle_id);
    let team_for_key = team_id.clone();
    let key = tauri::async_runtime::spawn_blocking(move || {
        certificates::load_signing_key(&SystemSecretStore, &team_for_key)
    })
    .await
    .map_err(|e| e.to_string())??;
    let mut certificate = None;
    if let Some(key) = key {
        for candidate in session
            .qh_list_certs(&team_id)
            .await
            .map_err(|e| e.to_string())?
            .certificates
        {
            let Some(der) = candidate.cert_content.as_ref() else {
                continue;
            };
            if !certificates::certificate_matches(&key, der.as_ref())? {
                continue;
            }
            let expires: std::time::SystemTime = candidate.expiration_date.clone().into();
            if expires <= std::time::SystemTime::now()
                || candidate.status.eq_ignore_ascii_case("revoked")
                || candidate.status.eq_ignore_ascii_case("expired")
            {
                continue;
            }
            if !candidate.status.eq_ignore_ascii_case("issued")
                && !candidate.status.eq_ignore_ascii_case("active")
            {
                return Err("无法确定现有签名证书的状态，请检查开发者账户".into());
            }
            certificate = Some(Certificate {
                serial: candidate.serial_number,
                name: candidate.name,
            });
            break;
        }
    }
    let mut input = PlanInput {
        app_name: info.name,
        original_bundle_id: info.bundle_id,
        requested_bundle_id: bundle_id,
        original_registered,
        requested_registered: false,
        team_id,
        team_name: team.name,
        device_id: device.id,
        device_name: device.name,
        device_registered,
        certificate,
    };
    let preliminary = build_plan(input.clone())?;
    input.requested_registered = app_ids
        .iter()
        .any(|item| item.identifier == preliminary.target_bundle_id);
    let plan = build_plan(input)?;
    if auth_state.generation() != generation {
        return Err("登录状态已变化，请重新准备签名".into());
    }
    let token = uuid::Uuid::new_v4().to_string();
    *plan_state.0.lock().map_err(|_| "签名计划不可用")? = Some(StoredPlan {
        token: token.clone(),
        plan: plan.clone(),
        ipa_path: path,
        digest,
        device_udid: device.udid,
        account_generation: generation,
    });
    Ok(Preview { token, plan })
}
