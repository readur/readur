# Review checklist

Report each line as done / not done in your final message.

1. **Tokens only** — `grep -nE "#[0-9a-fA-F]{3,8}\b|rgba?\(|border-radius:\s*[0-9]+px|z-index:\s*[0-9]" <your .module.css files>` returns nothing; no legacy aliases (`--focus`, `--signal`, `--ground`, `--btn-*`, `--danger-bg`); no `var(--x, fallback)` guesses.
2. **Primitives only** — no local card/badge/chip/spinner/status markup; every focus ring comes from a primitive.
3. **No colour-only meaning** — every coloured mark has a word or icon; no body text coloured with status tokens.
4. **Light and dark** — viewed both (theme toggle or `/dev/ui`).
5. **Phone width** — viewed at 390px: no horizontal page scroll, long names truncate.
6. **Keyboard** — every action reachable and visibly focused.
7. **Names and roles** — existing accessible names, roles and visible copy unchanged unless the task asked.
8. **i18n** — new strings added to `public/locales/{en,de,es,fr}/translation.json`.
9. **Reduced motion** — new keyframes stop under `prefers-reduced-motion`.
10. **Checks** — from `frontend/`: `npm run type-check` and `npm run test:unit` pass; `node ../scripts/contrast-check.mjs` passes if tokens changed.
