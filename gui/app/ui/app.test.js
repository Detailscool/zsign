import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mount } from './workflow.js';

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
