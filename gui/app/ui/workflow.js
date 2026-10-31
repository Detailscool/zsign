export function mount(document, openDialog, fetchDevices, auth, prepareSigning) {
  let ipa = '';
  let revision = 0;
  const output = document.querySelector('#ipa');
  const error = document.querySelector('#error');
  const invalidatePlan = () => {
    revision += 1;
    const area = document.querySelector('#plan-area');
    if (area) area.hidden = true;
  };
  document.querySelector('#choose').addEventListener('click', async () => {
    invalidatePlan();
    try {
      const selected = await openDialog();
      if (selected) { ipa = selected; invalidatePlan(); }
      output.textContent = ipa || '尚未选择 IPA';
      error.textContent = '';
    } catch (e) {
      ipa = '';
      invalidatePlan();
      output.textContent = '尚未选择 IPA';
      error.textContent = `无法选择文件：${e}`;
    }
  });
  document.querySelector('#refresh').addEventListener('click', async () => {
    invalidatePlan();
    try {
      const devices = await fetchDevices();
      const options = devices.map(device => {
        const option = document.createElement('option');
        option.value = device.id;
        option.textContent = device.name;
        return option;
      });
      if (!options.length) {
        const option = document.createElement('option');
        option.value = '';
        option.textContent = '尚未发现设备';
        options.push(option);
      }
      document.querySelector('#devices').replaceChildren(...options);
      invalidatePlan();
      error.textContent = '';
    } catch (e) {
      error.textContent = `无法发现设备：${e}`;
    }
  });
  document.querySelector('#devices')?.addEventListener('change', invalidatePlan);
  document.querySelector('#teams')?.addEventListener('change', invalidatePlan);
  document.querySelector('#bundle-id')?.addEventListener('input', invalidatePlan);
  document.querySelector('#prepare')?.addEventListener('click', async () => {
    invalidatePlan();
    const team = document.querySelector('#teams')?.value;
    const device = document.querySelector('#devices')?.value;
    if (!ipa || !team || !device) { error.textContent = '请选择 IPA、开发团队和目标设备。'; return; }
    const button = document.querySelector('#prepare');
    const preparedRevision = revision;
    button.disabled = true;
    error.textContent = '';
    try {
      const desired = document.querySelector('#bundle-id')?.value.trim() || null;
      const result = await prepareSigning(ipa, team, device, desired);
      if (revision !== preparedRevision) { error.textContent = '输入已变化，请重新检查。'; return; }
      const plan = result.plan;
      document.querySelector('#bundle-id').value = plan.target_bundle_id;
      document.querySelector('#plan-details').textContent = [
        `应用：${plan.app_name}`,
        `开发团队：${plan.team_name}`,
        `设备：${plan.device_name}`,
        `原 Bundle ID：${plan.original_bundle_id}`,
        `目标 Bundle ID：${plan.target_bundle_id}`,
        plan.register_device ? '将注册设备' : '复用已注册设备',
        plan.register_app_id ? '将创建 App ID' : '复用已有 App ID',
        plan.create_certificate ? '将创建证书' : `复用证书：${plan.certificate?.name || ''}`,
        '将获取或更新描述文件；不会自动撤销已有证书。',
      ].join('\n');
      document.querySelector('#plan-area').hidden = false;
    } catch (e) { error.textContent = `无法准备签名：${e}`; }
    finally { button.disabled = false; }
  });
  if (auth) {
    const status = document.querySelector('#auth-status');
    const verification = document.querySelector('#verification');
    const login = document.querySelector('#login');
    const loadTeams = async () => {
      const select = document.querySelector('#teams');
      if (!select || !auth.listTeams) return;
      try {
        const teams = await auth.listTeams();
        invalidatePlan();
        const options = teams.map(team => {
          const option = document.createElement('option');
          option.value = team.id;
          option.textContent = `${team.name} (${team.kind})`;
          return option;
        });
        if (!options.length) {
          const option = document.createElement('option');
          option.value = '';
          option.textContent = '没有可用的开发团队';
          options.push(option);
        }
        select.replaceChildren(...options);
        const area = document.querySelector('#team-area');
        if (area) area.hidden = false;
      } catch (e) { error.textContent = `无法读取开发团队：${e}`; }
    };
    const subscription = Promise.resolve(auth.subscribe(challenge => {
      verification.hidden = false;
      status.textContent = '等待验证码';
      const message = document.querySelector('#verification-message');
      if (message) message.textContent = challenge.method === 'sms' ? '请输入短信验证码。' : '请输入受信任设备上的验证码。';
      const phoneSelect = document.querySelector('#phones');
      const smsOptions = document.querySelector('#sms-options');
      if (phoneSelect && smsOptions) {
        const options = challenge.phones.map(phone => {
          const option = document.createElement('option');
          option.value = String(phone.id);
          option.textContent = `尾号 ${phone.last_two_digits}`;
          return option;
        });
        phoneSelect.replaceChildren(...options);
        smsOptions.hidden = options.length === 0;
      }
    }));
    login.addEventListener('click', async () => {
      invalidatePlan();
      const email = document.querySelector('#email').value.trim();
      const passwordInput = document.querySelector('#password');
      const password = passwordInput.value;
      const useRemote = document.querySelector('#remote')?.checked;
      const remoteUrl = useRemote ? document.querySelector('#remote-url').value.trim() : null;
      const remember = Boolean(document.querySelector('#remember')?.checked);
      passwordInput.value = '';
      const teamArea = document.querySelector('#team-area');
      if (teamArea) teamArea.hidden = true;
      document.querySelector('#teams')?.replaceChildren();
      login.disabled = true;
      error.textContent = '';
      status.textContent = '正在登录';
      try {
        await subscription;
        const result = await auth.login(email, password, remoteUrl, remember);
        verification.hidden = true;
        status.textContent = '已登录';
        const signOut = document.querySelector('#sign-out');
        if (signOut) signOut.hidden = false;
        if (teamArea) teamArea.hidden = false;
        await loadTeams();
        if (result?.warning) error.textContent = result.warning;
      } catch (e) {
        verification.hidden = true;
        status.textContent = '未登录';
        error.textContent = `登录失败：${e}`;
      } finally {
        login.disabled = false;
      }
    });
    document.querySelector('#verify').addEventListener('click', async () => {
      const input = document.querySelector('#code');
      try { await auth.submitCode(input.value); input.value = ''; error.textContent = ''; }
      catch (e) { error.textContent = `验证码未提交：${e}`; }
    });
    document.querySelector('#cancel-verify')?.addEventListener('click', async () => {
      try { await auth.cancel(); } catch (e) { error.textContent = `无法取消验证：${e}`; }
    });
    document.querySelector('#send-sms')?.addEventListener('click', async () => {
      try { await auth.sendSms(Number(document.querySelector('#phones').value)); }
      catch (e) { error.textContent = `无法发送短信：${e}`; }
    });
    document.querySelector('#refresh-teams')?.addEventListener('click', loadTeams);
    document.querySelector('#restore')?.addEventListener('click', async () => {
      invalidatePlan();
      const email = document.querySelector('#email').value.trim();
      const remoteUrl = document.querySelector('#remote')?.checked ? document.querySelector('#remote-url').value.trim() : null;
      login.disabled = true;
      error.textContent = '';
      status.textContent = '正在恢复会话';
      const teamArea = document.querySelector('#team-area');
      if (teamArea) teamArea.hidden = true;
      document.querySelector('#teams')?.replaceChildren();
      try {
        await auth.restore(email, remoteUrl);
        status.textContent = '已登录';
        const signOut = document.querySelector('#sign-out');
        if (signOut) signOut.hidden = false;
        if (teamArea) teamArea.hidden = false;
        await loadTeams();
      } catch (e) {
        status.textContent = '未登录';
        error.textContent = `无法恢复会话：${e}`;
      } finally { login.disabled = false; }
    });
    document.querySelector('#sign-out')?.addEventListener('click', async () => {
      invalidatePlan();
      try {
        const result = await auth.signOut();
        document.querySelector('#sign-out').hidden = !result?.warning;
        error.textContent = result?.warning ? `已退出，但无法清除保存的会话：${result.warning}` : '';
        status.textContent = '未登录';
        const area = document.querySelector('#team-area');
        if (area) area.hidden = true;
        document.querySelector('#teams')?.replaceChildren();
      } catch (e) { error.textContent = `无法退出登录：${e}`; }
    });
  }
}
