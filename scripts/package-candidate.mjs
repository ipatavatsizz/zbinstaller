import { createHash } from 'node:crypto';
import { lstatSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const applicationNames = [
  'ZombieBuddyInstaller-linux-amd64',
  'ZombieBuddyInstaller-macos-arm64.zip',
  'ZombieBuddyInstaller-windows-amd64.exe',
];

export function packageCandidate(directory) {
  const names = readdirSync(directory).filter((name) => name !== 'SHA256SUMS').sort();
  if (JSON.stringify(names) !== JSON.stringify(applicationNames)) {
    throw new Error(`Expected exactly the three application downloads, received: ${names.join(', ')}`);
  }
  const checksums = names.map((name) => {
    const filename = path.join(directory, name);
    if (!lstatSync(filename).isFile()) throw new Error(`Not a regular application file: ${name}`);
    const bytes = readFileSync(filename);
    const magic = name.endsWith('.exe') ? Buffer.from('MZ')
      : name.endsWith('.zip') ? Buffer.from([0x50, 0x4b, 0x03, 0x04]) : Buffer.from([0x7f, 0x45, 0x4c, 0x46]);
    if (!bytes.subarray(0, magic.length).equals(magic)) throw new Error(`Invalid application format: ${name}`);
    return `${createHash('sha256').update(bytes).digest('hex')}  ${name}\n`;
  }).join('');
  writeFileSync(path.join(directory, 'SHA256SUMS'), checksums);
  return checksums;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  if (!process.argv[2]) throw new Error('Usage: node scripts/package-candidate.mjs <application-directory>');
  process.stdout.write(packageCandidate(process.argv[2]));
}
