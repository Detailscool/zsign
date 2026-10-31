use crate::secrets::SystemSecretStore;
use omnisette::AnisetteConfiguration;
use plume_core::auth::{Account, TwoFactorAction, TwoFactorMethod, TwoFactorRequest};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc, Arc, Mutex,
};
use tauri::{Emitter, Manager, State};
use zsign_gui_core::sessions::{decode_session, encode_session, SessionVault};

#[derive(Default)]
pub struct Authentication {
    running: AtomicBool,
    pending: Mutex<Option<mpsc::Sender<Result<TwoFactorAction, String>>>>,
    pub account: Mutex<Option<Account>>,
    current_email: Mutex<Option<String>>,
    generation: AtomicU64,
}

impl Authentication {
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }
}

#[derive(Clone, Serialize)]
struct Phone {
    id: u32,
    last_two_digits: String,
}

#[derive(Clone, Serialize)]
struct Challenge {
    method: &'static str,
    phones: Vec<Phone>,
}

#[derive(Serialize)]
pub struct Team {
    id: String,
    name: String,
    kind: String,
}

#[derive(Serialize)]
pub struct LoginResult {
    warning: Option<String>,
}

#[derive(Serialize)]
pub struct SignOutResult {
    warning: Option<String>,
}

fn anisette_config(
    app: &tauri::AppHandle,
    remote_url: Option<String>,
) -> Result<AnisetteConfiguration, String> {
    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("anisette");
    let mut config = AnisetteConfiguration::new();
    if let Some(url) = remote_url {
        config = config
            .allow_remote(url.clone())
            .map_err(|e| e.to_string())?;
        directory = directory.join(format!("remote-{:x}", Sha256::digest(url.as_bytes())));
    } else {
        directory = directory.join("local");
    }
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    Ok(config.set_configuration_path(directory))
}

#[tauri::command]
pub async fn list_teams(state: State<'_, Arc<Authentication>>) -> Result<Vec<Team>, String> {
    let session = developer_session(&state).await?;
    let response = session.qh_list_teams().await.map_err(|e| e.to_string())?;
    Ok(response
        .teams
        .into_iter()
        .map(|team| Team {
            id: team.team_id,
            name: team.name,
            kind: team._type,
        })
        .collect())
}

pub async fn developer_session(
    state: &Authentication,
) -> Result<plume_core::developer::DeveloperSession, String> {
    let account = state
        .account
        .lock()
        .map_err(|_| "账户状态不可用")?
        .clone()
        .ok_or("请先登录 Apple ID")?;
    let spd = account.spd.as_ref().ok_or("登录会话不完整，请重新登录")?;
    if ["adsid", "GsIdmsToken", "sk", "c"]
        .iter()
        .any(|key| !spd.contains_key(*key))
    {
        return Err("登录会话不完整，请重新登录".into());
    }
    plume_core::developer::DeveloperSession::using_account(account)
        .await
        .map_err(|e| e.to_string())
}

struct RunningGuard(Arc<Authentication>);
impl Drop for RunningGuard {
    fn drop(&mut self) {
        if let Ok(mut pending) = self.0.pending.lock() {
            *pending = None;
        }
        self.0.running.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub async fn login(
    app: tauri::AppHandle,
    state: State<'_, Arc<Authentication>>,
    email: String,
    password: String,
    remote_url: Option<String>,
    remember: bool,
) -> Result<LoginResult, String> {
    if email.trim().is_empty() || password.is_empty() {
        return Err("请输入 Apple ID 和密码".into());
    }
    let config = anisette_config(&app, remote_url)?;
    if state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("登录正在进行中".into());
    }
    let shared = state.inner().clone();
    shared.generation.fetch_add(1, Ordering::AcqRel);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = RunningGuard(shared.clone());
        *shared.account.lock().map_err(|_| "账户状态不可用")? = None;
        *shared.current_email.lock().map_err(|_| "账户状态不可用")? = None;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let account = runtime
            .block_on(Account::login(
                || Ok((email.clone(), password.clone())),
                |request: TwoFactorRequest| {
                    let (tx, rx) = mpsc::channel();
                    *shared
                        .pending
                        .lock()
                        .map_err(|_| "验证码状态不可用".to_string())? = Some(tx);
                    let challenge = Challenge {
                        method: if request.method == TwoFactorMethod::Sms {
                            "sms"
                        } else {
                            "device"
                        },
                        phones: request
                            .trusted_phone_numbers
                            .iter()
                            .map(|p| Phone {
                                id: p.id,
                                last_two_digits: p.last_two_digits.clone(),
                            })
                            .collect(),
                    };
                    app.emit("authentication-challenge", challenge)
                        .map_err(|e| e.to_string())?;
                    let response = rx.recv().map_err(|_| "验证已取消".to_string())?;
                    *shared
                        .pending
                        .lock()
                        .map_err(|_| "验证码状态不可用".to_string())? = None;
                    response
                },
                config,
            ))
            .map_err(|e| e.to_string())?;
        let warning = if remember {
            let saved = account
                .spd
                .as_ref()
                .ok_or("登录会话不完整，无法保存".to_string())
                .and_then(encode_session)
                .and_then(|bytes| {
                    SessionVault::new(&SystemSecretStore).save_if_requested(&email, &bytes, true)
                });
            saved.err().map(|e| format!("已登录，但未能保存会话：{e}"))
        } else {
            None
        };
        *shared.account.lock().map_err(|_| "账户状态不可用")? = Some(account);
        *shared.current_email.lock().map_err(|_| "账户状态不可用")? = Some(email);
        Ok(LoginResult { warning })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn restore_session(
    app: tauri::AppHandle,
    state: State<'_, Arc<Authentication>>,
    email: String,
    remote_url: Option<String>,
) -> Result<(), String> {
    if email.trim().is_empty() {
        return Err("请输入 Apple ID".into());
    }
    let config = anisette_config(&app, remote_url)?;
    if state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("登录正在进行中".into());
    }
    let shared = state.inner().clone();
    shared.generation.fetch_add(1, Ordering::AcqRel);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = RunningGuard(shared.clone());
        *shared.account.lock().map_err(|_| "账户状态不可用")? = None;
        *shared.current_email.lock().map_err(|_| "账户状态不可用")? = None;
        let bytes = SessionVault::new(&SystemSecretStore)
            .load(&email)?
            .ok_or("没有找到已保存的会话")?;
        let spd = decode_session(&bytes)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let mut account = runtime
            .block_on(Account::new(config))
            .map_err(|e| e.to_string())?;
        account.spd = Some(spd);
        let session = runtime
            .block_on(plume_core::developer::DeveloperSession::using_account(
                account.clone(),
            ))
            .map_err(|_| "会话已失效或无法连接 Apple，请重新登录")?;
        runtime
            .block_on(session.qh_list_teams())
            .map_err(|_| "会话已失效或无法读取开发团队，请重新登录")?;
        *shared.account.lock().map_err(|_| "账户状态不可用")? = Some(account);
        *shared.current_email.lock().map_err(|_| "账户状态不可用")? = Some(email);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sign_out(state: State<'_, Arc<Authentication>>) -> Result<SignOutResult, String> {
    if state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("请先完成或取消登录".into());
    }
    let shared = state.inner().clone();
    shared.generation.fetch_add(1, Ordering::AcqRel);
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = RunningGuard(shared.clone());
        *shared.account.lock().map_err(|_| "账户状态不可用")? = None;
        let email = shared
            .current_email
            .lock()
            .map_err(|_| "账户状态不可用")?
            .clone();
        if let Some(email) = email {
            if let Err(error) = SessionVault::new(&SystemSecretStore).forget(&email) {
                return Ok(SignOutResult {
                    warning: Some(error),
                });
            }
        }
        *shared.current_email.lock().map_err(|_| "账户状态不可用")? = None;
        Ok(SignOutResult { warning: None })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn answer(state: &Authentication, action: Result<TwoFactorAction, String>) -> Result<(), String> {
    let sender = state
        .pending
        .lock()
        .map_err(|_| "验证码状态不可用")?
        .take()
        .ok_or("当前没有等待中的验证码请求")?;
    sender.send(action).map_err(|_| "验证请求已结束".into())
}

#[tauri::command]
pub fn submit_verification(
    state: State<'_, Arc<Authentication>>,
    code: String,
) -> Result<(), String> {
    let code = code.trim();
    if code.len() != 6 || !code.bytes().all(|c| c.is_ascii_digit()) {
        return Err("请输入 6 位数字验证码".into());
    }
    answer(&state, Ok(TwoFactorAction::SubmitCode(code.to_string())))
}

#[tauri::command]
pub fn send_sms(state: State<'_, Arc<Authentication>>, phone_id: u32) -> Result<(), String> {
    answer(&state, Ok(TwoFactorAction::SendSms(phone_id)))
}

#[tauri::command]
pub fn cancel_verification(state: State<'_, Arc<Authentication>>) -> Result<(), String> {
    answer(&state, Err("用户取消验证".into()))
}
