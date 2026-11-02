# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Two audiences, weighted equally:

- **Self-hosters** running Readur in a homelab for themselves or a household: archiving scans, bills, manuals, and paperwork. They mostly retrieve through search and occasionally tune sources, OCR, and server settings.
- **Small offices and teams**: several non-technical staff filing and retrieving business documents every day, plus one admin who configures sources, users, and OCR.

The interface has to stay approachable for the non-technical filer while still giving the admin every control.

## Product Purpose

Readur turns piles of scanned and digital documents into a searchable archive. Documents get in by upload, a watched folder, or a synced source (WebDAV, S3-compatible storage, local folders). Readur OCRs them (Tesseract and ocrmypdf, multi-language) and makes them findable through PostgreSQL full-text search. Success means someone finds the right document fast and trusts that everything they dropped in was processed.

## Positioning

Readur is self-hosted and owns the whole pipeline: ingest, OCR, full-text search, and organization all run in one Rust service on your own hardware. The document text is the product. Search results show matched OCR snippets, not just filenames.

## Operating Context

- Deployed with Docker Compose on the user's own server. The app may run offline, on a LAN, or air-gapped.
- Everyday loop: drop files in (upload, watch folder, or source sync) → OCR processes them → search / browse → open the document and read its OCR text → label it.
- Admin loop: configure sources and their health, watch-folder rules, OCR languages and concurrency, users and roles, API keys, ignored files, and failures.
- Sign-in: local accounts plus OIDC/SSO. Documents can be shared by link (SharedDocumentPage).
- Localized UI: en, de, es, fr (i18next). Copy length varies across languages.

## Capabilities and Constraints

- Frontend: React 19 + Vite + TypeScript. The component and theming library is open. The user allowed replacing MUI.
- **No runtime CDN or third-party calls.** Fonts and assets must be self-hosted and bundled.
- Test coverage after the redesign must be at least as good as before (638 unit tests at the start). Tests can be rewritten, but not weakened.
- Every page is in scope: Dashboard, Documents, Document details, Search, Upload, Sources, Watch Folder, Ignored Files, Labels, Document Management, Settings, Debug, Shared document, Login.
- Light and dark mode are both required. The user's choice is persisted.
- Document states the UI must communicate: pending / processing / completed / failed OCR, per-source sync health, and scan failures.

## Brand Commitments

- The name is "Readur". The existing logo assets are `frontend/public/readur*.png` and `favicon.ico`.
- The current indigo/Inter "Studio v3" look is not a commitment. The user asked for a fresh redesign.

## Evidence on Hand

- A real running instance at http://localhost:8123 (compose project `readur-uiaudit`). It starts empty. Demonstration documents have to be uploaded, and must not be presented as real customer data.
- No testimonials, customer names, or benchmarks exist. None may be invented.

## Product Principles

1. **Findability first.** Search and the document text are the center of gravity, and every page should shorten the path to "I found it".
2. **Trust through visible state.** Always show whether a document or source is processed, pending, or broken, and what to do about it.
3. **Calm for the filer, complete for the admin.** Everyday surfaces stay simple. Admin depth lives one step away and is never hidden.
4. **Yours, on your hardware.** Nothing phones home. The UI works the same offline.

## Accessibility & Inclusion

WCAG 2.2 AA is the floor: text contrast, visible keyboard focus, labeled controls, and a reduced-motion alternative. The audit of the previous build found AA contrast failures in its tokens, and those must not carry over.
