/**
 * Regression test: no hyphens in API endpoint paths.
 *
 * Style rule for this project: API paths use slashes, not dashes (e.g.
 * `/api/auth/keys`, not `/api/auth/api-keys`). This test greps the service
 * layer and direct API callers for any string literal that looks like an
 * API path and contains a `-`.
 *
 * The Rust-side counterpart is `tests/integration_api_contract_tests.rs`,
 * which enumerates every frontend endpoint and asserts each is registered
 * with the correct method.
 */

import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const REPO_ROOT = join(__dirname, '..', '..', '..', '..');

const FILES_TO_SCAN = [
  'frontend/src/services/api.ts',
  'frontend/src/pages/DocumentManagementPage.tsx',
  'frontend/src/pages/IgnoredFilesPage.tsx',
  'frontend/src/pages/SourcesPage.tsx',
  'frontend/src/pages/DebugPage.tsx',
];

// Matches string literals that look like API paths. We limit to paths that
// begin with `/api/` or — for service-layer calls that use an axios baseURL
// of `/api` — paths starting with `/` followed by a known router segment.
// React Router (UI) paths like `/ignored-files` are intentionally excluded.
const API_ROOT_SEGMENTS = [
  'auth', 'documents', 'ignored', 'labels', 'metrics', 'notifications',
  'ocr', 'queue', 'search', 'settings', 'source', 'sources', 'users',
  'webdav', 'shared', 'comments', 'health',
];
const segmentAlt = API_ROOT_SEGMENTS.join('|');
const PATH_LITERAL = new RegExp(
  `['"\`](\\/(?:api\\/)?(?:${segmentAlt})(?:\\/[a-zA-Z0-9_\${}/\\-:.?&=]*)?)['"\`]`,
  'g',
);

// Within a captured path, find any dash that is part of a segment (not inside
// a `${...}` placeholder). Segments are split on `/`.
function dashedSegments(path: string): string[] {
  const withoutPlaceholders = path.replace(/\$\{[^}]*\}/g, '');
  return withoutPlaceholders
    .split('/')
    .filter((segment) => segment.includes('-'))
    // Ignore query-string fragments like `?foo=bar-baz` — we only care about
    // the path portion.
    .map((segment) => segment.split('?')[0])
    .filter((segment) => segment.includes('-'));
}

describe('no-dashed-endpoints', () => {
  for (const relativePath of FILES_TO_SCAN) {
    it(`has no dashed API paths in ${relativePath}`, () => {
      const source = readFileSync(join(REPO_ROOT, relativePath), 'utf8');
      const offenders: string[] = [];

      let match: RegExpExecArray | null;
      PATH_LITERAL.lastIndex = 0;
      while ((match = PATH_LITERAL.exec(source)) !== null) {
        const literal = match[1];
        // Only flag paths that are plausibly API calls — they begin with
        // `/api/` or are single-slash paths that don't look like CSS
        // selectors or HTML IDs (heuristic: don't flag `/auth/callback` etc.
        // unless they contain a dash).
        if (!literal.startsWith('/')) continue;

        const dashed = dashedSegments(literal);
        if (dashed.length > 0) {
          offenders.push(literal);
        }
      }

      expect(offenders, `dashed API paths found in ${relativePath}:\n${offenders.join('\n')}`).toEqual([]);
    });
  }
});
