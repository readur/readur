import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const lock = JSON.parse(readFileSync(resolve(__dirname, '../../../package-lock.json'), 'utf8')) as {
  packages: Record<string, { resolved?: string }>;
};

describe('package-lock.json', () => {
  // CI and the Docker build install from the public registry; a tarball resolved from a local
  // mirror makes `npm install` fail there with ENOTFOUND.
  it('resolves every registry tarball from registry.npmjs.org', () => {
    const elsewhere = Object.entries(lock.packages)
      .filter(([, pkg]) => pkg.resolved?.endsWith('.tgz') && !pkg.resolved.startsWith('https://registry.npmjs.org/'))
      .map(([name, pkg]) => `${name} → ${pkg.resolved}`);
    expect(elsewhere).toEqual([]);
  });
});
