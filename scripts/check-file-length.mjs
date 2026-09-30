#!/usr/bin/env node
// File-length ratchet: no governed source file may reach MAX_LINES physical lines.
//
// Same convention as tyrfing's scripts/lint-file-length.py: a frozen baseline of
// path -> line count (scripts/file-length-baseline.json). A baselined file may
// shrink but never grow, and must be re-frozen (or removed) in the commit that
// shrinks it. Failure conditions, none advisory:
//
//   new offender  over the cap and not in the baseline
//   regression    over the cap, in the baseline, above its frozen count
//   slack         over the cap, in the baseline, below its frozen count
//   stale         at or under the cap but still in the baseline
//   unknown       in the baseline but matching no governed file
//
// Governed: frontend/src/**/*.{ts,tsx,css}, src/**/*.rs, tests/**/*.rs.
// Excluded: frontend/src/types/generated/ (ts-rs output, no human chose its length).
// Files are enumerated with `git ls-files` (independent of node_modules/target).
//
// Usage: node scripts/check-file-length.mjs [--update-baseline]

import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const MAX_LINES = 1000
const MIN_SCANNED = 200
const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const BASELINE_PATH = join(ROOT, 'scripts', 'file-length-baseline.json')
const GENERATED_PREFIXES = ['frontend/src/types/generated/']

function governed(path) {
  if (GENERATED_PREFIXES.some((p) => path.startsWith(p))) return false
  if (path.startsWith('frontend/src/')) return /\.(ts|tsx|css)$/.test(path)
  if (path.startsWith('src/') || path.startsWith('tests/')) return path.endsWith('.rs')
  return false
}

// Physical lines, like Python's len(text.splitlines()): comments and blanks count.
function lineCount(text) {
  if (text.length === 0) return 0
  const newlines = text.split('\n').length - 1
  return text.endsWith('\n') ? newlines : newlines + 1
}

function scan() {
  const tracked = execFileSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard'], {
    cwd: ROOT,
    maxBuffer: 64 * 1024 * 1024,
  })
    .toString()
    .split('\0')
    .filter(Boolean)
  const counts = {}
  for (const path of tracked) {
    if (!governed(path)) continue
    let text
    try {
      text = readFileSync(join(ROOT, path), 'utf8')
    } catch {
      continue // tracked but deleted in the working tree
    }
    counts[path] = lineCount(text)
  }
  return counts
}

const counts = scan()
const scanned = Object.keys(counts).length

if (process.argv.includes('--update-baseline')) {
  const over = Object.fromEntries(
    Object.entries(counts)
      .filter(([, n]) => n >= MAX_LINES)
      .sort(([a], [b]) => a.localeCompare(b)),
  )
  writeFileSync(BASELINE_PATH, JSON.stringify(over, null, 2) + '\n')
  console.log(`Wrote ${Object.keys(over).length} entries to scripts/file-length-baseline.json`)
  process.exit(0)
}

const baseline = JSON.parse(readFileSync(BASELINE_PATH, 'utf8'))
const failures = []

if (scanned < MIN_SCANNED) {
  failures.push(`only ${scanned} governed files scanned (floor ${MIN_SCANNED}); the walker is broken`)
}

for (const [path, n] of Object.entries(counts)) {
  if (n < MAX_LINES) continue
  const frozen = baseline[path]
  if (frozen === undefined) {
    failures.push(`new offender: ${path} has ${n} lines (cap is ${MAX_LINES - 1}); split it`)
  } else if (n > frozen) {
    failures.push(`regression: ${path} grew from ${frozen} to ${n} lines`)
  } else if (n < frozen) {
    failures.push(`slack: ${path} shrank to ${n} lines but is frozen at ${frozen}; lower its baseline entry to ${n}`)
  }
}

for (const [path, frozen] of Object.entries(baseline)) {
  const n = counts[path]
  if (n === undefined) {
    failures.push(`unknown: baseline entry ${path} matches no governed file; remove it`)
  } else if (n < MAX_LINES) {
    failures.push(`stale: ${path} is now ${n} lines (under the cap) but still baselined at ${frozen}; remove the entry`)
  }
}

if (failures.length > 0) {
  console.error(`File-length ratchet failed (${failures.length}):`)
  for (const f of failures) console.error(`  - ${f}`)
  process.exit(1)
}

console.log(
  `File-length ratchet OK: ${scanned} files scanned, ${Object.keys(baseline).length} baselined over ${MAX_LINES - 1} lines.`,
)
