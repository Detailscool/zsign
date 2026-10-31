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
