use crate::{
    auth::{self, Authentication},
    certificates, devices, ipa, package,
    preparation::{hash_file, PlanStore},
    signer,
};
use rsa::pkcs8::{EncodePrivateKey, LineEnding};
use serde::Serialize;
use std::{io::Write, sync::Arc};
use tauri::{Emitter, State};

#[derive(Clone, Serialize)]
struct Progress {
    message: String,
}

fn progress(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit(
        "signing-progress",
        Progress {
            message: message.into(),
        },
    );
}

fn decode_profile(bytes: &[u8]) -> Result<plist::Dictionary, String> {
    let start = bytes
        .windows(6)
        .position(|w| w == b"<plist")
        .ok_or("描述文件内容无效")?;
    let end = bytes
        .windows(8)
        .rposition(|w| w == b"</plist>")
        .ok_or("描述文件内容无效")?
        + 8;
    if end <= start {
        return Err("描述文件内容无效".into());
    }
    plist::from_bytes(&bytes[start..end]).map_err(|_| "描述文件内容无效".into())
}

#[tauri::command]
pub async fn execute_signing(
    app: tauri::AppHandle,
    auth_state: State<'_, Arc<Authentication>>,
    plan_state: State<'_, Arc<PlanStore>>,
    token: String,
) -> Result<String, String> {
    let plan = plan_state.consume(&token, auth_state.generation())?;
    let cli = signer::cli_path(&app)?;
    let export = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .add_filter("iOS App", &["ipa"])
            .set_file_name("signed.ipa")
            .save_file()
    })
    .await
    .map_err(|e| e.to_string())?
    .ok_or("已取消保存")?;
    if export.exists() {
        return Err("输出文件已存在，请选择新的文件名".into());
    }
    if export == plan.ipa_path {
        return Err("输出文件不能覆盖原始 IPA".into());
    }
    let workspace = tempfile::tempdir().map_err(|e| e.to_string())?;
    let snapshot = workspace.path().join("input.ipa");
    std::fs::copy(&plan.ipa_path, &snapshot).map_err(|e| format!("无法读取 IPA：{e}"))?;
    if hash_file(&snapshot)? != plan.digest {
        return Err("IPA 已改变，请重新预览签名计划".into());
    }
    let info = ipa::inspect_ipa(&snapshot)?;
    let device = devices::discover()?
        .into_iter()
        .find(|d| d.id == plan.plan.device_id && d.udid == plan.device_udid)
        .ok_or("目标设备已断开，请重新预览")?;
    if auth_state.generation() != plan.account_generation {
        return Err("登录状态已变化，请重新预览".into());
    }
    let session = auth::developer_session(&auth_state).await?;
    let team = &plan.plan.team_id;
    progress(&app, "检查设备和开发团队…");
    let registered = session
        .qh_get_device(team, &device.udid)
        .await
        .map_err(|e| e.to_string())?;
    if registered.is_none() {
        if !plan.plan.register_device {
            return Err("设备状态已变化，请重新预览".into());
        }
        session
            .qh_add_device(team, &device.name, &device.udid)
            .await
            .map_err(|e| format!("注册设备失败：{e}"))?;
    } else if !registered.as_ref().is_some_and(|d| {
        d.status.eq_ignore_ascii_case("enabled") || d.status.eq_ignore_ascii_case("active")
    }) {
        return Err("开发者账户中的设备未启用".into());
    }
    progress(&app, "准备开发证书…");
    let (key, cert) =
        certificates::prepare_certificate(&session, team, plan.plan.create_certificate).await?;
    let target = &plan.plan.target_bundle_id;
    progress(&app, "准备 App ID 和描述文件…");
    let app_id = match session
        .qh_get_app_id(team, target)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(id) => id,
        None if plan.plan.register_app_id => {
            session
                .qh_add_app_id(team, &plan.plan.app_name, target)
                .await
                .map_err(|e| format!("创建 App ID 失败：{e}"))?
                .app_id
        }
        None => return Err("App ID 状态已变化，请重新预览".into()),
    };
    let profile = session
        .qh_get_profile(team, &app_id.app_id_id)
        .await
        .map_err(|e| format!("获取描述文件失败：{e}"))?
        .provisioning_profile
        .encoded_profile;
    let profile_bytes = profile.as_ref();
    let dictionary = decode_profile(profile_bytes)?;
    zsign_gui_core::profile::validate_profile(&dictionary, team, target, &device.udid, &cert)?;
    if auth_state.generation() != plan.account_generation {
        return Err("登录状态已变化，请重新预览".into());
    }
    progress(&app, "正在签名 IPA…");
    let root = workspace.path().join("content");
    package::extract(&snapshot, &root)?;
    let app_path = root.join(&info.app_path);
    let modes = package::capture_permissions(&app_path)?;
    let mut key_file =
        tempfile::NamedTempFile::new_in(workspace.path()).map_err(|e| e.to_string())?;
    let pem = key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|_| "无法编码签名私钥")?;
    key_file
        .write_all(pem.as_bytes())
        .map_err(|e| e.to_string())?;
    let profile_path = workspace.path().join("profile.mobileprovision");
    std::fs::write(&profile_path, profile_bytes).map_err(|e| e.to_string())?;
    signer::run_cli(
        &cli,
        &[
            "-f".into(),
            "-k".into(),
            key_file.path().as_os_str().into(),
            "-m".into(),
            profile_path.as_os_str().into(),
            "-b".into(),
            target.into(),
            app_path.as_os_str().into(),
        ],
        workspace.path(),
    )?;
    drop(key_file);
    package::restore_permissions(&app_path, &modes)?;
    let parent = export.parent().ok_or("输出路径无效")?;
    let tmp =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| format!("无法创建输出文件：{e}"))?;
    package::archive(&root, tmp.path())?;
    tmp.persist_noclobber(&export)
        .map_err(|e| format!("无法保存 IPA：{}", e.error))?;
    progress(&app, "签名完成，正在安装到设备…");
    #[cfg(target_os = "macos")]
    {
        let output = devices::devicectl()?
            .args(["device", "install", "app", "--device", &device.id])
            .arg(&app_path)
            .output()
            .map_err(|e| format!("IPA 已保存至 {}，但安装无法启动：{e}", export.display()))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "IPA 已保存至 {}，但安装失败：{}",
                export.display(),
                detail.chars().take(1200).collect::<String>()
            ));
        }
    }
    #[cfg(not(target_os = "macos"))]
    return Err(format!(
        "IPA 已保存至 {}，此平台的安装功能尚未完成",
        export.display()
    ));
    #[cfg(target_os = "macos")]
    Ok(export.to_string_lossy().into_owned())
}
