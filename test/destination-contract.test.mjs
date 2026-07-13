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
    { primitive: 'Fluent Button', count: 1 },
    { primitive: 'Fluent Menu', count: 1 },
    { primitive: 'Fluent MenuItem', count: 1 },
    { primitive: 'Bebop icon', count: 1 },
    { primitive: 'Native textarea', count: 1 },
  ]);
});

test('requires native delivery and rejects reconstruction embedding', () => {
  const contract = destinationContract();
  assert.equal(contract.mode, 'native-required');
  assert.deepEqual(contract.requiredPackages, [
    '@1js/bebop-icons',
    '@1js/fluentui-modern',
  ]);
  assert.ok(contract.forbiddenDelivery.includes('iframe embedding'));
});
