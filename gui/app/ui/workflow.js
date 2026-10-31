export function mount(document, openDialog) {
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
}
