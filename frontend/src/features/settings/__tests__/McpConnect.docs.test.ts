import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { claudeCodeCommand, claudeDesktopJson, mcpServersJson } from '../apiKeys/McpConnect';

// docs/mcp-guide.md shows the same client configuration the settings panel generates. Keep them
// identical so the documented setup is the one that works (e.g. Claude Desktop needs mcp-remote).
const guide = readFileSync(resolve(__dirname, '../../../../../docs/mcp-guide.md'), 'utf8');
const ENDPOINT = 'https://readur.example.com/api/mcp';
const KEY = 'readur_pat_XXXXXXXX';

/** The fenced block of `lang` that follows `heading` in the guide. */
function blockAfter(heading: string, lang: string): string {
  const start = guide.indexOf(heading);
  expect(start, `heading "${heading}" in docs/mcp-guide.md`).toBeGreaterThanOrEqual(0);
  const fence = guide.indexOf('```' + lang, start);
  const body = guide.indexOf('\n', fence) + 1;
  return guide.slice(body, guide.indexOf('```', body));
}

describe('docs/mcp-guide.md client setup', () => {
  it('Claude Code command matches the settings panel', () => {
    const documented = blockAfter('#### Claude Code', 'bash')
      .replace(/\\\n\s*/g, '')
      .trim();
    expect(documented).toBe(claudeCodeCommand(ENDPOINT, KEY));
  });

  it('Claude Desktop config matches the settings panel', () => {
    expect(JSON.parse(blockAfter('#### Claude Desktop', 'json'))).toEqual(
      JSON.parse(claudeDesktopJson(ENDPOINT, KEY)),
    );
  });

  it('native HTTP config matches the settings panel', () => {
    expect(JSON.parse(blockAfter('#### Clients with native HTTP support', 'json'))).toEqual(
      JSON.parse(mcpServersJson(ENDPOINT, KEY)),
    );
  });
});
