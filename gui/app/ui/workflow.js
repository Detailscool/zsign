export function mount(document, openDialog, fetchDevices) {
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
}
