export function mount(document, openDialog, fetchDevices, auth) {
  let ipa = '';
  const output = document.querySelector('#ipa');
  const error = document.querySelector('#error');
  document.querySelector('#choose').addEventListener('click', async () => {
    try {
      const selected = await openDialog();
      if (selected) ipa = selected;
      output.textContent = ipa || '尚未选择 IPA';
      error.textContent = '';
    } catch (e) {
      error.textContent = `无法选择文件：${e}`;
    }
  });
  document.querySelector('#refresh').addEventListener('click', async () => {
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
      error.textContent = '';
    } catch (e) {
      error.textContent = `无法发现设备：${e}`;
    }
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
      const email = document.querySelector('#email').value.trim();
      const passwordInput = document.querySelector('#password');
      const password = passwordInput.value;
      const useRemote = document.querySelector('#remote')?.checked;
      const remoteUrl = useRemote ? document.querySelector('#remote-url').value.trim() : null;
      passwordInput.value = '';
      const teamArea = document.querySelector('#team-area');
      if (teamArea) teamArea.hidden = true;
      document.querySelector('#teams')?.replaceChildren();
      login.disabled = true;
      error.textContent = '';
      status.textContent = '正在登录';
      try {
        await subscription;
        await auth.login(email, password, remoteUrl);
        verification.hidden = true;
        status.textContent = '已登录';
        if (teamArea) teamArea.hidden = false;
        await loadTeams();
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
  }
}
