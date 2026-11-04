import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { applicationNames, packageCandidate } from '../../scripts/package-candidate.mjs';

function fixture(run) {
  const root = mkdtempSync(path.join(tmpdir(), 'zb-release-'));
  const files = [Buffer.from([0x7f, 0x45, 0x4c, 0x46, 1]), Buffer.from([0x50, 0x4b, 0x03, 0x04, 2]), Buffer.from('MZ executable')];
  try {
    applicationNames.forEach((name, index) => writeFileSync(path.join(root, name), files[index]));
    run(root, files);
  } finally { rmSync(root, { recursive: true, force: true }); }
}

test('only the three application files receive stable SHA-256 entries', () => fixture((root, files) => {
  const checksum = packageCandidate(root);
  assert.equal(checksum, applicationNames.map((name, index) => `${createHash('sha256').update(files[index]).digest('hex')}  ${name}\n`).join(''));
  assert.equal(readFileSync(path.join(root, 'SHA256SUMS'), 'utf8'), checksum);
  assert.equal(packageCandidate(root), checksum);
}));

test('reject missing application downloads', () => fixture((root) => {
  rmSync(path.join(root, applicationNames[0]));
  assert.throws(() => packageCandidate(root), /exactly the three/);
}));

test('reject screenshot artifacts accidentally merged into a release', () => fixture((root) => {
  writeFileSync(path.join(root, 'main.png'), 'screenshot');
  assert.throws(() => packageCandidate(root), /exactly the three/);
}));

test('reject a renamed error page masquerading as an application', () => fixture((root) => {
  writeFileSync(path.join(root, applicationNames[2]), '<html>error</html>');
  assert.throws(() => packageCandidate(root), /Invalid application format/);
}));
