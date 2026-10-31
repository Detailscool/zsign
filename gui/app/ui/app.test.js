import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mount } from './workflow.js';

test('verification opens a modal and dismissing it cancels sign in', async () => {
  const ids = ['#ipa', '#error', '#choose', '#refresh', '#devices', '#login', '#email', '#password', '#auth-status', '#verification', '#code', '#verify', '#cancel-verify'];
  const elements = Object.fromEntries(ids.map(id => [id, {
    textContent: '', value: '', hidden: false, handlers: {},
    addEventListener(event, handler) { this.handlers[event] = handler; }, replaceChildren() {}
  }]));
  const dialog = elements['#verification'];
  dialog.showModal = () => { dialog.open = true; };
  dialog.close = () => { dialog.open = false; };
  let challenge, cancelLogin, cancelled = false;
  const auth = {
    subscribe: handler => { challenge = handler; },
    login: () => { challenge({ method: 'device', phones: [] }); return new Promise((_, reject) => { cancelLogin = reject; }); },
    cancel: async () => { cancelled = true; cancelLogin('已取消'); }
  };
  mount({ querySelector: id => elements[id] }, async () => null, async () => [], auth);
  const login = elements['#login'].handlers.click();
  await Promise.resolve();
  assert.equal(dialog.open, true);
  let prevented = false;
  await dialog.handlers.cancel({ preventDefault() { prevented = true; } });
  await login;
  assert.equal(prevented, true);
  assert.equal(cancelled, true);
  assert.equal(dialog.open, false);
  assert.equal(elements['#auth-status'].textContent, '未登录');
});

test('selecting an IPA displays its path to the user', async () => {
  const elements = Object.fromEntries(['#ipa', '#error', '#choose', '#refresh', '#devices'].map(id => [id, {
    textContent: '', addEventListener(_event, handler) { this.click = handler; }
  }]));
  mount({ querySelector: id => elements[id] }, async () => '/Users/test/Example.ipa');
  await elements['#choose'].click();
  assert.equal(elements['#ipa'].textContent, '/Users/test/Example.ipa');
});

test('a rejected IPA clears an earlier selection', async () => {
  const elements = Object.fromEntries(['#ipa', '#error', '#choose', '#refresh', '#devices'].map(id => [id, {
    textContent: '', addEventListener(_event, handler) { this.click = handler; }
  }]));
  let calls = 0;
  mount({ querySelector: id => elements[id] }, async () => {
    if (++calls === 1) return '/Users/test/Example.ipa';
    throw new Error('不支持扩展');
  });
  await elements['#choose'].click();
  await elements['#choose'].click();
  assert.equal(elements['#ipa'].textContent, '尚未选择 IPA');
  assert.match(elements['#error'].textContent, /不支持扩展/);
});

test('device refresh displays connected device names', async () => {
  const elements = Object.fromEntries(['#ipa', '#error', '#choose', '#refresh', '#devices'].map(id => [id, {
    textContent: '', addEventListener(_event, handler) { this.click = handler; },
    replaceChildren(...children) { this.children = children; }
  }]));
  const document = { querySelector: id => elements[id], createElement: () => ({}) };
  mount(document, async () => null, async () => [{ id: 'device-1', name: 'My iPhone' }]);
  await elements['#refresh'].click();
  assert.equal(elements['#devices'].children[0].textContent, 'My iPhone');
});

test('a verification challenge asks for a code and submits it to the authentication service', async () => {
  const ids = ['#ipa', '#error', '#choose', '#refresh', '#devices', '#login', '#email', '#password', '#auth-status', '#verification', '#code', '#verify'];
  const elements = Object.fromEntries(ids.map(id => [id, {
    textContent: '', value: '', hidden: false,
    addEventListener(_event, handler) { this.click = handler; }, replaceChildren() {}
  }]));
  elements['#email'].value = 'user@example.com';
  elements['#password'].value = 'secret';
  let challenge, code, finishLogin;
  const auth = {
    subscribe: handler => { challenge = handler; },
    login: () => { challenge({ method: 'device', phones: [] }); return new Promise(resolve => { finishLogin = resolve; }); },
    submitCode: async value => { code = value; finishLogin(); },
  };
  mount({ querySelector: id => elements[id] }, async () => null, async () => [], auth);
  const login = elements['#login'].click();
  await Promise.resolve();
  assert.equal(elements['#verification'].hidden, false);
  assert.equal(elements['#password'].value, '');
  elements['#code'].value = '123456';
  await elements['#verify'].click();
  await login;
  assert.equal(code, '123456');
});

test('after signing in the user can choose a developer team', async () => {
  const ids = ['#ipa', '#error', '#choose', '#refresh', '#devices', '#login', '#email', '#password', '#auth-status', '#verification', '#code', '#verify', '#teams'];
  const elements = Object.fromEntries(ids.map(id => [id, {
    textContent: '', value: '', hidden: false,
    addEventListener(_event, handler) { this.click = handler; },
    replaceChildren(...children) { this.children = children; }
  }]));
  elements['#email'].value = 'user@example.com';
  elements['#password'].value = 'secret';
  const document = { querySelector: id => elements[id], createElement: () => ({}) };
  const auth = { subscribe() {}, login: async () => {}, listTeams: async () => [{ id: 'TEAM1', name: 'Test Team', kind: 'Free' }] };
  mount(document, async () => null, async () => [], auth);
  await elements['#login'].click();
  assert.equal(elements['#teams'].children[0].textContent, 'Test Team (Free)');
});

test('the remember-session choice is sent to the login service', async () => {
  const ids = ['#ipa', '#error', '#choose', '#refresh', '#devices', '#login', '#email', '#password', '#auth-status', '#verification', '#code', '#verify', '#remember'];
  const elements = Object.fromEntries(ids.map(id => [id, {
    textContent: '', value: '', checked: false, hidden: false,
    addEventListener(_event, handler) { this.click = handler; }, replaceChildren() {}
  }]));
  elements['#email'].value = 'user@example.com';
  elements['#password'].value = 'secret';
  elements['#remember'].checked = true;
  let remembered;
  const auth = { subscribe() {}, login: async (_email, _password, _remote, remember) => { remembered = remember; } };
  mount({ querySelector: id => elements[id] }, async () => null, async () => [], auth);
  await elements['#login'].click();
  assert.equal(remembered, true);
});

test('a saved session can be restored without entering a password', async () => {
  const ids = ['#ipa', '#error', '#choose', '#refresh', '#devices', '#login', '#email', '#password', '#auth-status', '#verification', '#code', '#verify', '#restore'];
  const elements = Object.fromEntries(ids.map(id => [id, {
    textContent: '', value: '', checked: false, hidden: false,
    addEventListener(_event, handler) { this.click = handler; }, replaceChildren() {}
  }]));
  elements['#email'].value = 'user@example.com';
  let restored;
  const auth = { subscribe() {}, restore: async email => { restored = email; } };
  mount({ querySelector: id => elements[id] }, async () => null, async () => [], auth);
  await elements['#restore'].click();
  assert.equal(restored, 'user@example.com');
  assert.equal(elements['#auth-status'].textContent, '已登录');
});

test('signing preparation presents the account changes before any installation', async () => {
  const ids = ['#ipa', '#error', '#choose', '#refresh', '#devices', '#login', '#email', '#password', '#auth-status', '#verification', '#code', '#verify', '#teams', '#bundle-id', '#prepare', '#plan-details', '#plan-area'];
  const elements = Object.fromEntries(ids.map(id => [id, {
    textContent: '', value: '', checked: false, hidden: false,
    addEventListener(_event, handler) { this.click = handler; }, replaceChildren() {}
  }]));
  elements['#email'].value = 'user@example.com';
  elements['#password'].value = 'secret';
  elements['#teams'].value = 'TEAM123456';
  elements['#devices'].value = 'device-1';
  const auth = { subscribe() {}, login: async () => {} };
  const prepare = async () => ({ plan: { app_name: 'Example', original_bundle_id: 'com.example.app', target_bundle_id: 'com.zsign.example', team_name: 'Test Team', device_name: 'iPhone', register_device: true, register_app_id: true, create_certificate: true } });
  mount({ querySelector: id => elements[id] }, async () => '/tmp/Example.ipa', async () => [], auth, prepare);
  await elements['#choose'].click();
  await elements['#login'].click();
  await elements['#prepare'].click();
  assert.match(elements['#plan-details'].textContent, /注册设备/);
  assert.match(elements['#plan-details'].textContent, /创建证书/);
  assert.equal(elements['#plan-area'].hidden, false);
});

test('a preview is discarded if the inputs change while it is loading', async () => {
  const ids = ['#ipa', '#error', '#choose', '#refresh', '#devices', '#login', '#email', '#password', '#auth-status', '#verification', '#code', '#verify', '#teams', '#bundle-id', '#prepare', '#plan-details', '#plan-area'];
  const elements = Object.fromEntries(ids.map(id => [id, {
    textContent: '', value: '', checked: false, hidden: false,
    addEventListener(_event, handler) { this.click = handler; }, replaceChildren() {}
  }]));
  elements['#email'].value = 'user@example.com'; elements['#password'].value = 'secret';
  elements['#teams'].value = 'TEAM123456'; elements['#devices'].value = 'device-1';
  let finish;
  const prepare = () => new Promise(resolve => { finish = resolve; });
  mount({ querySelector: id => elements[id] }, async () => '/tmp/Example.ipa', async () => [], { subscribe() {}, login: async () => {} }, prepare);
  await elements['#choose'].click(); await elements['#login'].click();
  const pending = elements['#prepare'].click();
  elements['#bundle-id'].value = 'com.changed.app'; elements['#bundle-id'].click();
  finish({ plan: { app_name: 'old', target_bundle_id: 'com.old.app' } });
  await pending;
  assert.equal(elements['#plan-area'].hidden, true);
});

test('confirming a preview starts exactly that prepared signing task', async () => {
  const ids = ['#ipa', '#error', '#choose', '#refresh', '#devices', '#login', '#email', '#password', '#auth-status', '#verification', '#code', '#verify', '#teams', '#bundle-id', '#prepare', '#plan-details', '#plan-area', '#execute', '#task-status'];
  const elements = Object.fromEntries(ids.map(id => [id, { textContent: '', value: '', hidden: false,
    addEventListener(_event, handler) { this.click = handler; }, replaceChildren() {} }]));
  elements['#teams'].value = 'TEAM123456'; elements['#devices'].value = 'device-1';
  let token;
  const signing = { execute: async value => { token = value; return '/tmp/signed.ipa'; }, subscribe() {} };
  const prepare = async () => ({ token: 'approved-plan', plan: { app_name: 'App', original_bundle_id: 'com.example.app', target_bundle_id: 'com.example.app', team_name: 'Team', device_name: 'Phone' } });
  mount({ querySelector: id => elements[id] }, async () => '/tmp/input.ipa', async () => [], { subscribe() {} }, prepare, signing);
  await elements['#choose'].click(); await elements['#prepare'].click(); await elements['#execute'].click();
  assert.equal(token, 'approved-plan');
  assert.match(elements['#task-status'].textContent, /signed.ipa/);
});
