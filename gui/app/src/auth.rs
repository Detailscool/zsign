use omnisette::AnisetteConfiguration;
use plume_core::auth::{Account, TwoFactorAction, TwoFactorMethod, TwoFactorRequest};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use tauri::{Emitter, Manager, State};

#[derive(Default)]
pub struct Authentication {
    running: AtomicBool,
    pending: Mutex<Option<mpsc::Sender<Result<TwoFactorAction, String>>>>,
    pub account: Mutex<Option<Account>>,
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

#[tauri::command]
pub async fn list_teams(state: State<'_, Arc<Authentication>>) -> Result<Vec<Team>, String> {
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
    let session = plume_core::developer::DeveloperSession::using_account(account)
        .await
        .map_err(|e| e.to_string())?;
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
) -> Result<(), String> {
    if email.trim().is_empty() || password.is_empty() {
        return Err("请输入 Apple ID 和密码".into());
    }
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("anisette");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut config = AnisetteConfiguration::new().set_configuration_path(directory);
    if let Some(url) = remote_url {
        config = config.allow_remote(url).map_err(|e| e.to_string())?;
    }
    if state
        .running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("登录正在进行中".into());
    }
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = RunningGuard(shared.clone());
        *shared.account.lock().map_err(|_| "账户状态不可用")? = None;
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
        *shared.account.lock().map_err(|_| "账户状态不可用")? = Some(account);
        Ok(())
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
