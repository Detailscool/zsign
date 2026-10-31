import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { mount } from './workflow.js';

test('selecting an IPA displays its path to the user', async () => {
  const elements = Object.fromEntries(['#ipa', '#error', '#choose'].map(id => [id, {
    textContent: '', addEventListener(_event, handler) { this.click = handler; }
  }]));
  mount({ querySelector: id => elements[id] }, async () => '/Users/test/Example.ipa');
  await elements['#choose'].click();
  assert.equal(elements['#ipa'].textContent, '/Users/test/Example.ipa');
});
