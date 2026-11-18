import test from 'node:test';
import assert from 'node:assert/strict';
import { getGreeting } from './greeting';

test('getGreeting returns exact Hello World text', () => {
  assert.equal(getGreeting(), 'Hello World');
});
