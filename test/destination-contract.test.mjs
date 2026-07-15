import assert from 'node:assert/strict';
import test from 'node:test';
import {
  buildPrimitiveInventory,
  destinationContract,
} from '../src/destination-contract.mjs';

test('maps captured controls to destination primitive hints', () => {
  assert.deepEqual(buildPrimitiveInventory([
    { tag: 'button' },
    { role: 'menu' },
    { role: 'menuitem' },
    { tag: 'svg' },
    { tag: 'textarea' },
  ]), [
    { primitive: 'Native design-system button', count: 1 },
    { primitive: 'Native design-system menu', count: 1 },
    { primitive: 'Native design-system menu item', count: 1 },
    { primitive: 'Native design-system icon', count: 1 },
    { primitive: 'Native textarea', count: 1 },
  ]);
});

test('requires native delivery and rejects reconstruction embedding', () => {
  const contract = destinationContract();
  assert.equal(contract.mode, 'native-required');
  assert.deepEqual(contract.requiredPackages, []);
  assert.ok(contract.forbiddenDelivery.includes('iframe embedding'));
});
