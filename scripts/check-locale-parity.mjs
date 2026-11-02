#!/usr/bin/env node
// Fails unless en, de, es and fr have an identical key set. Keys are flattened
// to dotted paths, so nested namespaces are compared leaf by leaf. en is the
// reference; missing and extra keys are listed per locale.
//
// Usage: node scripts/check-locale-parity.mjs

import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const LOCALES_DIR = join(ROOT, 'frontend', 'public', 'locales')
const REFERENCE = 'en'
const LOCALES = ['en', 'de', 'es', 'fr']

function flatten(value, prefix = '', out = new Set()) {
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
    for (const [k, v] of Object.entries(value)) flatten(v, prefix ? `${prefix}.${k}` : k, out)
  } else {
    out.add(prefix)
  }
  return out
}

const keys = {}
for (const locale of LOCALES) {
  const file = join(LOCALES_DIR, locale, 'translation.json')
  keys[locale] = flatten(JSON.parse(readFileSync(file, 'utf8')))
}

let failed = false
for (const locale of LOCALES) {
  if (locale === REFERENCE) continue
  const missing = [...keys[REFERENCE]].filter((k) => !keys[locale].has(k)).sort()
  const extra = [...keys[locale]].filter((k) => !keys[REFERENCE].has(k)).sort()
  if (missing.length === 0 && extra.length === 0) continue
  failed = true
  console.error(`${locale}: ${missing.length} missing, ${extra.length} extra (vs ${REFERENCE})`)
  for (const k of missing) console.error(`  missing  ${k}`)
  for (const k of extra) console.error(`  extra    ${k}`)
}

if (failed) process.exit(1)
console.log(`Locale parity OK: ${LOCALES.join(', ')} share ${keys[REFERENCE].size} keys.`)
