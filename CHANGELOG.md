# Changelog

## [3.1.3](https://github.com/readur/readur/compare/v3.1.2...v3.1.3) (2026-11-06)


### Miscellaneous Chores

* **github-action:** Update action github/codeql-action (v4.39.1 ➔ v4.39.2) ([#1024](https://github.com/readur/readur/issues/1024)) ([64900a8](https://github.com/readur/readur/commit/64900a82bf6df71d670043081d7cc1a22e996c69))
* **github-action:** Update action jdx/mise-action (v5.2.5 ➔ v5.2.6) ([#1020](https://github.com/readur/readur/issues/1020)) ([ea5c95a](https://github.com/readur/readur/commit/ea5c95a00d374c1fa00a06e950e5a1d63f1c6b7f))
* **mise:** Update tool aqua:zizmorcore/zizmor (1.31.1 ➔ 1.32.0) ([#1023](https://github.com/readur/readur/issues/1023)) ([58cb945](https://github.com/readur/readur/commit/58cb94537f09abad7f6e6370f83bb2daa5e8f9c7))
* **npm:** Update dependency i18next-browser-languagedetector (8.3.0 ➔ 8.3.0) ([#1018](https://github.com/readur/readur/issues/1018)) ([dbb0f1c](https://github.com/readur/readur/commit/dbb0f1c21015f3b4d94daa75b117e1c2847302de))
* **npm:** Update dependency i18next-http-backend (4.0.9 ➔ 4.0.10) ([#1026](https://github.com/readur/readur/issues/1026)) ([e5aa3ff](https://github.com/readur/readur/commit/e5aa3ffe5a6d22f55632c04aa6c259297fe16eaf))
* **npm:** Update dependency lucide-react (1.66.0 ➔ 1.67.0) ([#1025](https://github.com/readur/readur/issues/1025)) ([518bee7](https://github.com/readur/readur/commit/518bee71a1d88293b28cc2cbf137d23e47fb6398))
* **npm:** Update dependency react-i18next (17.1.11 ➔ 17.1.12) ([#1027](https://github.com/readur/readur/issues/1027)) ([c5d90b6](https://github.com/readur/readur/commit/c5d90b6ddf1be4aa3883c72e8df795beca329227))
* **rust:** Update crate zip (9.2.2 ➔ 9.3.0) ([#1022](https://github.com/readur/readur/issues/1022)) ([48bb727](https://github.com/readur/readur/commit/48bb727ac0129b781fa261be19713f34eb419f70))

## [3.1.2](https://github.com/readur/readur/compare/v3.1.1...v3.1.2) (2026-11-04)


### Bug Fixes

* **sync:** harden remote-source synchronization (WebDAV/Nextcloud, S3, local folder) ([#1016](https://github.com/readur/readur/issues/1016)) ([91e68d6](https://github.com/readur/readur/commit/91e68d6cd5f3740b4ecdd0f8cfc24929ba248249))


### Miscellaneous Chores

* **github-action:** Update github-actions ([#1014](https://github.com/readur/readur/issues/1014)) ([89e8fb1](https://github.com/readur/readur/commit/89e8fb1786d7c59dfdcc75b4fde2383c1402b672))
* **renovate-config:** Update config home-operations/renovate-presets (10.0.1 ➔ 11.0.0) ([#1015](https://github.com/readur/readur/issues/1015)) ([1aa24da](https://github.com/readur/readur/commit/1aa24da9f899617523dfee7f4d512083ef7de1a5))

## [3.1.1](https://github.com/readur/readur/compare/v3.1.0...v3.1.1) (2026-11-03)


### Bug Fixes

* **ci:** build leptonica-sys bindings against leptonica 1.86 on ubuntu-26.04 ([83ac6e6](https://github.com/readur/readur/commit/83ac6e6278c765c9db3637ffca93e0de9d334a41))
* **ci:** get main green again (file-length ratchet, stale TS bindings) ([0f1047f](https://github.com/readur/readur/commit/0f1047f26f23e07bab86b297532557e554569ab3))
* **ci:** get main green again (file-length ratchet, stale TS bindings) ([e69b09a](https://github.com/readur/readur/commit/e69b09a7e380d2ce23a5508c644cb0cd6221fb1e))
* **ci:** keep formatters off generated TS bindings; restore ts-rs Label.ts ([013944d](https://github.com/readur/readur/commit/013944dc42f0ff1d21a4c6aa8faceedc9feab255))
* **db:** strip U+0000 from every text/JSON bind in one place ([64e8a38](https://github.com/readur/readur/commit/64e8a383360df847d3f75a46c97827e2a132959c))
* **db:** strip U+0000 from every text/JSON bind in one place ([bcee4a6](https://github.com/readur/readur/commit/bcee4a603943074cd5602004b38eac5381df1197)), closes [#901](https://github.com/readur/readur/issues/901)
* **docs:** unblock the docs build (MkDocs-2 notice, pip cache poisoning) ([3762904](https://github.com/readur/readur/commit/37629049c6efe38152bdf9d462c4fd68d8a876f5))
* **docs:** unblock the docs build (MkDocs-2 notice, pip cache poisoning) ([e2df3a3](https://github.com/readur/readur/commit/e2df3a3133ff577187c1b7d6afc02ce43a1b6cb3))
* **ocr:** honour CONCURRENT_OCR_JOBS for the OCR worker ([3c52429](https://github.com/readur/readur/commit/3c5242969ced8e52dda625b4019520b0207cdd8f))
* **ocr:** honour CONCURRENT_OCR_JOBS for the OCR worker ([3142619](https://github.com/readur/readur/commit/3142619a5418a95f2e7149d044a62080a83fdc6d))
* **ocr:** sanitize the OCR queue's error and processed-image binds ([e3323e9](https://github.com/readur/readur/commit/e3323e976766bd4dedae6f1d8246d65a90a41608)), closes [#901](https://github.com/readur/readur/issues/901)
* **ocr:** stop skipping pages 1-9 when OCRing PDFs with 10+ pages ([f53861e](https://github.com/readur/readur/commit/f53861ed6f33e905c33f3c771d037c264ca39be3))
* **ocr:** stop skipping pages 1-9 when OCRing PDFs with 10+ pages ([3e5c9e4](https://github.com/readur/readur/commit/3e5c9e4498e2385d659109701ee4e748143770e2)), closes [#980](https://github.com/readur/readur/issues/980)
* render multipage PDF thumbnails without padded suffixes ([e63a660](https://github.com/readur/readur/commit/e63a6605dcfb42460326ffe5f119f5afed102c9c))
* render multipage PDF thumbnails without padded suffixes ([de37446](https://github.com/readur/readur/commit/de3744648902846fd6832e893706bec37aae5587))
* **renovate:** follow home-operations preset repo rename; inline local policy ([ee988ce](https://github.com/readur/readur/commit/ee988cecac306a62e7ecf57a1aaa3ef434715b5e))
* **renovate:** follow home-operations preset repo rename; inline local policy ([4fdc5ee](https://github.com/readur/readur/commit/4fdc5eeaa9632311fd1df3f3a2eb8fae2553b091)), closes [#986](https://github.com/readur/readur/issues/986)
* **security:** resolve open CodeQL alerts in workflows and OCR retry stats ([33a2d2c](https://github.com/readur/readur/commit/33a2d2c056f8f85f5fa0f8c41d9248ded3ffbccc))
* **security:** resolve open CodeQL alerts in workflows and OCR retry stats ([5a4a364](https://github.com/readur/readur/commit/5a4a3644ceee90e3715788c7d771659ae443f06e))


### Documentation

* point dev guides at the paths that exist now ([74a4abe](https://github.com/readur/readur/commit/74a4abe1bf086482a301c714246b33c4c34239db))


### Styles

* **ocr:** rustfmt the touched ocr modules (format on touch) ([655e12a](https://github.com/readur/readur/commit/655e12acfe68a22cd83a2cc740e518a6b5a831ba))


### Tests

* postgres:18 and per-test databases cloned from a migrated template ([fed7f60](https://github.com/readur/readur/commit/fed7f60d1f7b04bbf56f8bc97feff2db7302a5b5))
* postgres:18 and per-test databases cloned from a migrated template ([08fe6b4](https://github.com/readur/readur/commit/08fe6b41155ea5c7cc81ccf43a2a2d0522cda3c0))
* test_helpers' database helpers also clone the template ([ef1fe5b](https://github.com/readur/readur/commit/ef1fe5b2cfafba0714c263bea4e0930e386edd53))
* **webdav:** expect U+0000 stripped from stored etags ([18482c8](https://github.com/readur/readur/commit/18482c87f8c2e3550c5f6796bf595d83372756e4)), closes [#901](https://github.com/readur/readur/issues/901)


### Build System

* **deps:** testcontainers 0.27 and testcontainers-modules 0.15 ([349f1b9](https://github.com/readur/readur/commit/349f1b919f4a34ceeb799b70f55f8c7ab8ead07d))


### Continuous Integration

* **docs:** skip the Cloudflare preview deploy on fork PRs ([3466ad0](https://github.com/readur/readur/commit/3466ad0b525d58041d91db293507e5ec4b6caf88))
* **docs:** skip the Cloudflare preview deploy on fork PRs ([5c8b341](https://github.com/readur/readur/commit/5c8b341dc71b2e181d6d21d8f9b9aee0d4b7f60e))


### Miscellaneous Chores

* **container:** Update image sigoden/dufs (v0.46.0 ➔ v0.47.0) ([#996](https://github.com/readur/readur/issues/996)) ([d969e71](https://github.com/readur/readur/commit/d969e713166fcf713b9be787bee3f3b33e218a9b))
* **db:** remove uncompiled src/db_guardrails.rs ([8336434](https://github.com/readur/readur/commit/833643408846e630a70d97862a88925f415fc614)), closes [#901](https://github.com/readur/readur/issues/901)
* **deps:** Lock file maintenance (npm) ([#883](https://github.com/readur/readur/issues/883)) ([77cc810](https://github.com/readur/readur/commit/77cc8102d4b70bc59ca65927243ee09ac7245b04))
* **github-action:** Update github-actions ([#862](https://github.com/readur/readur/issues/862)) ([6c237f4](https://github.com/readur/readur/commit/6c237f48f9fbca0b288ec8bf9e8cebb5af2e3d9e))
* **github-action:** Update github-actions ([#960](https://github.com/readur/readur/issues/960)) ([401fdd4](https://github.com/readur/readur/commit/401fdd4ab30da2847fc6f1206a2af92b48054d99))
* **helm:** Update chart common (4.6.2 ➔ 5.3.0) ([#675](https://github.com/readur/readur/issues/675)) ([39cc346](https://github.com/readur/readur/commit/39cc34637d804cc5ece4a9147aeb87e36436d4f6))
* **mise:** Update mise tools ([#968](https://github.com/readur/readur/issues/968)) ([0115b14](https://github.com/readur/readur/commit/0115b1445f61c5ddcce13865f294bf9b35cf0bf4))
* **mise:** Update tool node (24.21.0 ➔ v24.22.2) ([#919](https://github.com/readur/readur/issues/919)) ([058368a](https://github.com/readur/readur/commit/058368af252cd959317f74749458c6280e5d85ba))
* **mise:** Update tool node (24.22.2 ➔ v26.13.0) ([#985](https://github.com/readur/readur/issues/985)) ([3bba8bd](https://github.com/readur/readur/commit/3bba8bdaa0f37766ff6eec8313e95dd2d35c352c))
* **mise:** Update tool rust ([e66a20a](https://github.com/readur/readur/commit/e66a20a6811f1a4dd4ef388bae488475761c7712))
* **mise:** Update tool rust ([aa26a98](https://github.com/readur/readur/commit/aa26a9886ecab3114b820a022b60de11ccd0675a))
* **npm:** Update dependency @playwright/test (1.64.0 ➔ 1.64.1) ([#974](https://github.com/readur/readur/issues/974)) ([e984510](https://github.com/readur/readur/commit/e98451004c0553fbee0a4fce2426aabf6ef00fb8))
* **npm:** Update dependency @testing-library/jest-dom (6.9.1 ➔ 7.0.1) ([#791](https://github.com/readur/readur/issues/791)) ([63f6454](https://github.com/readur/readur/commit/63f6454187c95a7b5f973108c6a9a9ebb46908ff))
* **npm:** Update dependency @testing-library/user-event (14.6.7 ➔ 14.6.8) ([#969](https://github.com/readur/readur/issues/969)) ([01dbf4c](https://github.com/readur/readur/commit/01dbf4c1394955cdeaa495fe52c6ea62ae209313))
* **npm:** Update dependency @vitejs/plugin-react (5.2.0 ➔ 6.2.1) ([#588](https://github.com/readur/readur/issues/588)) ([887a5c8](https://github.com/readur/readur/commit/887a5c826c3169bbabacd4db05495ef2494bf3eb))
* **npm:** Update dependency axios (1.15.0 ➔ 1.21.0) ([#847](https://github.com/readur/readur/issues/847)) ([120c989](https://github.com/readur/readur/commit/120c9890916c176136d55fb8879ac4b668407220))
* **npm:** Update dependency i18next (25.10.10 ➔ 26.7.1) ([#618](https://github.com/readur/readur/issues/618)) ([6b89462](https://github.com/readur/readur/commit/6b89462878a8d5cbe7070818838d4e330895cb38))
* **npm:** Update dependency i18next-browser-languagedetector (8.2.1 ➔ 8.2.4) ([#983](https://github.com/readur/readur/issues/983)) ([a81c0cf](https://github.com/readur/readur/commit/a81c0cfa59d90bbf6bd5717124563de4fdbef2da))
* **npm:** Update dependency i18next-http-backend (3.0.6 ➔ 4.0.9) ([#674](https://github.com/readur/readur/issues/674)) ([e3d6df4](https://github.com/readur/readur/commit/e3d6df4c39a07326135ddfa82868e238b2fc582f))
* **npm:** Update dependency jsdom (29.1.1 ➔ 30.1.3) ([#807](https://github.com/readur/readur/issues/807)) ([024e951](https://github.com/readur/readur/commit/024e9514b9d6be8cc2b92ddafb7c6cb945e997f4))
* **npm:** Update dependency lucide-react (1.55.0 ➔ 1.65.0) ([#971](https://github.com/readur/readur/issues/971)) ([a16abfe](https://github.com/readur/readur/commit/a16abfea6a1d1e8a4480b6ff2c8e3a4b0bc015c7))
* **npm:** Update dependency lucide-react (1.66.0 ➔ 1.66.0) ([#1013](https://github.com/readur/readur/issues/1013)) ([203ded9](https://github.com/readur/readur/commit/203ded9c199bbe372b6520dde6d7595f912952f0))
* **npm:** Update dependency react-aria-components (1.21.1 ➔ 1.22.1) ([#950](https://github.com/readur/readur/issues/950)) ([0049692](https://github.com/readur/readur/commit/00496921acee83dd6065dce021224add7f941610))
* **npm:** Update dependency react-dropzone (15.0.0 ➔ 20.1.4) ([#815](https://github.com/readur/readur/issues/815)) ([c45cd56](https://github.com/readur/readur/commit/c45cd56a6af9fc0b760adf13d3d5f6c608ce4ee4))
* **npm:** Update dependency react-i18next (16.6.6 ➔ 17.1.11) ([#617](https://github.com/readur/readur/issues/617)) ([a78f5a0](https://github.com/readur/readur/commit/a78f5a07508daa451c935e9487520e0210e5aca7))
* **npm:** Update dependency react-router-dom (7.18.4 ➔ 7.18.5) ([#995](https://github.com/readur/readur/issues/995)) ([709bcac](https://github.com/readur/readur/commit/709bcac5cf46d86104bee3eb2de5475164738325))
* **npm:** Update dependency typescript (5.9.3 ➔ 7.0.2) ([#757](https://github.com/readur/readur/issues/757)) ([fe9c94e](https://github.com/readur/readur/commit/fe9c94e377307e54735d7cce88f6bfeb055802a9))
* **npm:** Update dependency uuid (13.0.2 ➔ 14.0.3) ([#652](https://github.com/readur/readur/issues/652)) ([e4fd93f](https://github.com/readur/readur/commit/e4fd93f5e16819d4b3dc7d298d84658a48fe0b64))
* **npm:** Update dependency vite (7.3.7 ➔ 8.3.5) ([#589](https://github.com/readur/readur/issues/589)) ([b61dc15](https://github.com/readur/readur/commit/b61dc159d0f1a29e70634f38eb5c56ba9564aa27))
* **npm:** Update react monorepo (19.3.0 ➔ 19.3.1) ([#976](https://github.com/readur/readur/issues/976)) ([c13d3ea](https://github.com/readur/readur/commit/c13d3eabf2ee2a4f013bb019db6c28dfb7c6d98f))
* **npm:** Update vitest monorepo (3.2.7 ➔ 5.1.0) ([#16](https://github.com/readur/readur/issues/16)) ([ed7fe11](https://github.com/readur/readur/commit/ed7fe11d23ec7bef42d8251b4c0ab577d3ebe429))
* **renovate:** group testcontainers with testcontainers-modules ([fc00821](https://github.com/readur/readur/commit/fc008213d3e2a7083c63a77b9a77ba885b0b5add))
* **renovate:** group testcontainers with testcontainers-modules ([728ee7b](https://github.com/readur/readur/commit/728ee7b2b00df42e473daca2e94d6fc5ac7ce246))
* **rust:** Update aws-sdk-rust monorepo ([#981](https://github.com/readur/readur/issues/981)) ([94b5e38](https://github.com/readur/readur/commit/94b5e38ed0303abb13b2ade9008c4d0a8ecf6730))
* **rust:** Update crate imageproc (0.25.1 ➔ 0.27.0) ([#997](https://github.com/readur/readur/issues/997)) ([7cadb16](https://github.com/readur/readur/commit/7cadb1681a3195c68c0acbe27e57dbcc2acaa57b))
* **rust:** Update crate jsonwebtoken (9.3.1 ➔ 11.1.0) ([#804](https://github.com/readur/readur/issues/804)) ([91a2a6c](https://github.com/readur/readur/commit/91a2a6cf11b131a940e4e04c5e07a8336f89bb4b))
* **rust:** Update crate libc (0.2.189 ➔ 0.2.191) ([#1009](https://github.com/readur/readur/issues/1009)) ([a45c225](https://github.com/readur/readur/commit/a45c225b9e3e7cbd8122b76d369ff14a1c96a378))
* **rust:** Update crate notify (8.2.0 ➔ 8.2.1) ([#984](https://github.com/readur/readur/issues/984)) ([3586079](https://github.com/readur/readur/commit/35860799def63b9ca5ac7790240dfeffbd446885))
* **rust:** Update crate quick-xml (0.37.5 ➔ 0.42.0) ([#998](https://github.com/readur/readur/issues/998)) ([0d3a049](https://github.com/readur/readur/commit/0d3a049dcaf2279633f82e2aec4cb76012674beb))
* **rust:** Update crate rand (0.9.5 ➔ 0.10.3) ([#999](https://github.com/readur/readur/issues/999)) ([3167e40](https://github.com/readur/readur/commit/3167e40a65c7c7ab9d382b9e1d71ee9fa278014b))
* **rust:** Update crate reqwest (0.12.28 ➔ 0.13.5) ([#1000](https://github.com/readur/readur/issues/1000)) ([a1030f2](https://github.com/readur/readur/commit/a1030f2bb43b97b71a988393a51f6479738af894))
* **rust:** Update crate rust_xlsxwriter (0.99.1 ➔ 0.102.0) ([#1001](https://github.com/readur/readur/issues/1001)) ([ecda56b](https://github.com/readur/readur/commit/ecda56bf934dd9d646e328471aa20d2a0425c8dc))
* **rust:** Update crate serde_json (1.0.152 ➔ 1.0.154) ([#965](https://github.com/readur/readur/issues/965)) ([7b68647](https://github.com/readur/readur/commit/7b68647106dbea8d5433097de09eddcc5ea4e233))
* **rust:** Update crate sha2 (0.10.9 ➔ 0.11.1) ([#1002](https://github.com/readur/readur/issues/1002)) ([179c785](https://github.com/readur/readur/commit/179c7851f8c60fc4567a86a8cf2cf5777fcad23c))
* **rust:** Update crate sha2 (0.11.0 ➔ 0.11.1) ([#1007](https://github.com/readur/readur/issues/1007)) ([b1b2393](https://github.com/readur/readur/commit/b1b23931f8fa0bb55167a6cb929cc10f08ba2f0d))
* **rust:** Update crate sqlx (0.8.6 ➔ 0.9.0) ([#1003](https://github.com/readur/readur/issues/1003)) ([92ca0be](https://github.com/readur/readur/commit/92ca0be0b27660548da42cb00731037658946702))
* **rust:** Update crate sysinfo (0.39.6 ➔ 0.40.1) ([#1004](https://github.com/readur/readur/issues/1004)) ([ca4f7d6](https://github.com/readur/readur/commit/ca4f7d69e49ebb03a99587040fa1bb943f5dd3d9))
* **rust:** Update crate tokio (1.53.2 ➔ 1.54.1) ([#972](https://github.com/readur/readur/issues/972)) ([4f70820](https://github.com/readur/readur/commit/4f708206d01948ed619d4c0e0b637893a229d909))
* **rust:** Update crate uuid (1.28.0 ➔ 1.29.0) ([#982](https://github.com/readur/readur/issues/982)) ([5381a23](https://github.com/readur/readur/commit/5381a23438b60348145785004199015c035b73dc))
* **rust:** Update crate zip (8.6.0 ➔ 9.2.2) ([#946](https://github.com/readur/readur/issues/946)) ([84cb6be](https://github.com/readur/readur/commit/84cb6bee36c528456951c1addc2d2923ed4ef1ef))
* **rust:** Update utoipa monorepo ([#908](https://github.com/readur/readur/issues/908)) ([e7146e5](https://github.com/readur/readur/commit/e7146e548e2ab4e208d88d4677984907b9f42270))

## [3.1.0](https://github.com/readur/readur/compare/v3.0.0...v3.1.0) (2026-10-13)


### Features

* **container:** update image rust (1.98 → 1.99) ([#923](https://github.com/readur/readur/issues/923)) ([eb29c1f](https://github.com/readur/readur/commit/eb29c1f9c1c7836956b2568a0c069deee2f963a6))
* **dev:** primitive gallery at /dev/ui; fix SourceBadge tile class collision ([78d6af0](https://github.com/readur/readur/commit/78d6af0955a33de7f689b7fff1332d41f12b38fd))
* **document:** clicking an image in the drawer opens it full size in a new tab ([32eebad](https://github.com/readur/readur/commit/32eebad91df2c85815aa94880f7c0fdf50fadbdd))
* **document:** document drawer — head, labels saved at once, preview on top, Text/Details/Comments/Share tabs ([7bc02ca](https://github.com/readur/readur/commit/7bc02ca1968923e5ff69948d21a28a39bdbc1c2e))
* **document:** drag the dotted handle between the file and the tabs to resize them (remembered) ([9a1a275](https://github.com/readur/readur/commit/9a1a275976ec5f8f0b06b329b1241bfcd89da5c2))
* **document:** drawer context — list registration, ?document= opener, change event ([ccca3c5](https://github.com/readur/readur/commit/ccca3c553dc508da1a83692fe12c50ff2ace1c57))
* **document:** remove a label straight from its chip ([5d2699c](https://github.com/readur/readur/commit/5d2699cba1380496484a14a6ac9837ec77568132))
* **document:** the drawer's actions sit in a bar along its bottom ([9fcf152](https://github.com/readur/readur/commit/9fcf15290deeb1ae2e175a2455d058d65df50409))
* every document link opens the drawer over the current page ([75a73a9](https://github.com/readur/readur/commit/75a73a9aebef8716960e289562e735a058d8f39e))
* **home:** mockup layout — pipeline, needs attention, sources, just arrived ([220fb08](https://github.com/readur/readur/commit/220fb08df3aca2faa26a6b721288515dc73e74cf))
* **intake:** failed-OCR and import-failure details follow ?failure= and ?import= ([8f527a5](https://github.com/readur/readur/commit/8f527a5ee8ad0d7c78f1ecaf0c4172d97e5b5338))
* **labels:** every account owns its default labels; no more system labels ([86ba225](https://github.com/readur/readur/commit/86ba225e017d68394a3987cdd9adbd920dd48435))
* **labels:** one kind of label in the UI; every label can be edited, deleted and detached ([f18c97a](https://github.com/readur/readur/commit/f18c97a5550fd68de3a836b5eb1eab33a84cd826))
* **library:** clicking the preview in the detail drawer opens the document ([4a9cc78](https://github.com/readur/readur/commit/4a9cc78b4020271bd3c4fb112bb4255031cd219f))
* **library:** Library and Search open documents in the global drawer ([d3d35a6](https://github.com/readur/readur/commit/d3d35a6414c5d2b79fc89bc530459e1f219dea58))
* **library:** thumbnail-first document cards with metadata, live OCR status ([120755d](https://github.com/readur/readur/commit/120755daa0be0f18914f7eb521066b5a9fe5eccd))
* **lib:** useDrawerParam keeps an open drawer in the URL ([6286457](https://github.com/readur/readur/commit/62864578fcf8f640088cb89dc1bffadaf2b48a90))
* **npm:** update dependency @playwright/test (1.63.0 → 1.64.0) ([#949](https://github.com/readur/readur/issues/949)) ([70f15bc](https://github.com/readur/readur/commit/70f15bcc9567057f79864ed30422f5c399c33945))
* **npm:** update dependency lucide-react (1.48.0 → 1.49.0) ([#925](https://github.com/readur/readur/issues/925)) ([dca6c9d](https://github.com/readur/readur/commit/dca6c9def36b669fad625d7d71445550d2dcbb4e))
* **npm:** update dependency lucide-react (1.49.0 → 1.48.0) ([#918](https://github.com/readur/readur/issues/918)) ([6cb8e68](https://github.com/readur/readur/commit/6cb8e68039c50dff9121511c6f7c5762ccac10c2))
* **npm:** update dependency lucide-react (1.49.0 → 1.50.0) ([#932](https://github.com/readur/readur/issues/932)) ([645ae6f](https://github.com/readur/readur/commit/645ae6fee9f2a0747112113e95f4bdc158e6c236))
* **npm:** update dependency lucide-react (1.50.0 → 1.51.0) ([#935](https://github.com/readur/readur/issues/935)) ([e4fea08](https://github.com/readur/readur/commit/e4fea08f21380cd4e804e55511cea601b18d776b))
* **npm:** update dependency lucide-react (1.51.0 → 1.52.0) ([#939](https://github.com/readur/readur/issues/939)) ([8570ff3](https://github.com/readur/readur/commit/8570ff340935283f801691c24d6ed5d5c23f1d5a))
* **npm:** update dependency lucide-react (1.52.0 → 1.53.0) ([#951](https://github.com/readur/readur/issues/951)) ([75e3556](https://github.com/readur/readur/commit/75e35560ba5c31dd8707ac67790b83b4d34e9fe2))
* **npm:** update dependency lucide-react (1.53.0 → 1.54.0) ([#954](https://github.com/readur/readur/issues/954)) ([1a9d26b](https://github.com/readur/readur/commit/1a9d26bcf1eec213882f9ae5fa8caf0d40d25620))
* **npm:** update dependency lucide-react (1.54.0 → 1.55.0) ([#957](https://github.com/readur/readur/issues/957)) ([df56160](https://github.com/readur/readur/commit/df56160cbf2b5a24ac15517ce6e02b60d4ca3f04))
* **pwa:** installable app — manifest, iOS home-screen tags, square icons, safe areas ([d5e9d3e](https://github.com/readur/readur/commit/d5e9d3e5fb38ed07e6d7a63603cc5f95442393b3))
* **routes:** /documents/:id redirects to the drawer; remove the document page ([dd08ae1](https://github.com/readur/readur/commit/dd08ae14064e600d0a337e5f7cebcaf5d43c4a63))
* **rust:** update aws-sdk-rust monorepo ([#926](https://github.com/readur/readur/issues/926)) ([7d5b7ec](https://github.com/readur/readur/commit/7d5b7ecc6343023fd101a08bc89de2d389711283))
* **rust:** update crate aws-sdk-s3 (1.151.0 → 1.152.0) ([#930](https://github.com/readur/readur/issues/930)) ([7b06b0b](https://github.com/readur/readur/commit/7b06b0bbdb412a7d15cfb112f18f1da6376d466e))
* **rust:** update crate dav-server (0.10.0 → 0.11.0) ([#849](https://github.com/readur/readur/issues/849)) ([14603af](https://github.com/readur/readur/commit/14603af56de173cb814367973a75aa1d6dab728a))
* **rust:** update crate hyper (1.11.1 → 1.12.0) ([#947](https://github.com/readur/readur/issues/947)) ([7f49c23](https://github.com/readur/readur/commit/7f49c23ca2321a8c2a3ac2e56e97cfe7e8e51504))
* **rust:** update crate infer (0.19.0 → 0.22.0) ([#851](https://github.com/readur/readur/issues/851)) ([f9ab240](https://github.com/readur/readur/commit/f9ab2408a1ea10c1bda51a2201450ddbdcf527a5))
* **rust:** update crate rust_xlsxwriter (0.94.1 → 0.99.1) ([#855](https://github.com/readur/readur/issues/855)) ([5682e4e](https://github.com/readur/readur/commit/5682e4e8604f29f9971d80378de9f6f4f5bd029a))
* **rust:** update crate uuid (1.26.1 → 1.27.0) ([#934](https://github.com/readur/readur/issues/934)) ([cad3f73](https://github.com/readur/readur/commit/cad3f731c0bd0321ebae091e2f9e98fd527e421f))
* **rust:** update crate uuid (1.27.0 → 1.28.0) ([#956](https://github.com/readur/readur/issues/956)) ([e2dd3a5](https://github.com/readur/readur/commit/e2dd3a5d4ccc14922e3baab43862d3e2d45ca18a))
* **shell:** a progress toast follows every running sync ([4ea59aa](https://github.com/readur/readur/commit/4ea59aa0b412f1136f7f3762b1f185d4b67e282c))
* **shell:** document drawer opens over any page from ?document= ([4f99862](https://github.com/readur/readur/commit/4f99862529eaacb097a6ea3ffdfc94644ed6eef1))
* **shell:** drag the phone menu open with a sideways swipe anywhere; it follows the finger ([ecfb7d6](https://github.com/readur/readur/commit/ecfb7d6c0b2a952070f6724620d9038786db886e))
* **shell:** library count in the nav, source icon tiles, roomier user row ([2435487](https://github.com/readur/readur/commit/243548765976cc886272e47bb5acda942373418f))
* **shell:** the Search destination is now "Advanced search" (nav and page title) ([e022561](https://github.com/readur/readur/commit/e022561c9eb8921ea57083a483855c4db842cfae))
* **sources:** connection details follow ?source= — rows write it, Back closes, unknown ids drop ([65a83eb](https://github.com/readur/readur/commit/65a83eb3c04a53831fc32ef34062b7b0b318ae7b))
* **sources:** give connections and the watch folder their own Sources page ([5811a7b](https://github.com/readur/readur/commit/5811a7b9f526f32f83819f2d58dd5a449e26659d))
* **theme:** Indigo & Amber scheme; warning moves to orange, src-1/src-8 clear the new accent ([c81f7f3](https://github.com/readur/readur/commit/c81f7f3c902104d71aca90e730acb6ad414e478a))
* **ui:** bigger grab areas for the drawer's resize handles, same look ([f974bee](https://github.com/readur/readur/commit/f974beed5c9d9ff9c6bf42bf560da025c55742cc))
* **ui:** ChoiceTile/ChoiceGroup, M3 switch, Studio checkbox ([918496e](https://github.com/readur/readur/commit/918496ef914c9ad51cc4345731f7656e45de5d92))
* **ui:** complete rebuild ([aba6a41](https://github.com/readur/readur/commit/aba6a41e6fd22cfe93326caabce8f83bc1269268))
* **ui:** contained BoardTable with zebra and accent selection edge ([502f1fb](https://github.com/readur/readur/commit/502f1fbffa51a3ef71d7f4dbef8d0c90f3f2dd40))
* **ui:** DocumentCard primitive (thumbnail-first) ([d6fdc83](https://github.com/readur/readur/commit/d6fdc8363a828ef40682db0e71b5ef1fc5548d92))
* **ui:** dot/spinner StatusMark, quiet ChangeTag and Kbd ([ac6b709](https://github.com/readur/readur/commit/ac6b709d98269e36430e714a391df1090af4bbe2))
* **ui:** floating slide-over, spotlight palette with preview, surface bulk bar ([69273f7](https://github.com/readur/readur/commit/69273f7c969ab3f606b55a5b67bdd50fba1ea148))
* **ui:** illustrated empty state, shimmer skeleton, numbered pagination ([aee7616](https://github.com/readur/readur/commit/aee7616fd309943b029408ac614985f324f33f18))
* **ui:** one Notice primitive (stripe card); striped toasts with timer ([d270be2](https://github.com/readur/readur/commit/d270be2fcca8709de9c09fcd18091342ae6e1c8a))
* **ui:** outlined Studio fields and tinted-selection listbox ([225a37d](https://github.com/readur/readur/commit/225a37d1109ef71d7133f4cd1939eb821b4de749))
* **ui:** ProgressBar and OutcomeBar primitives ([eaa6f97](https://github.com/readur/readur/commit/eaa6f97a91d522ef3734aacbb47582daa7293b7e))
* **ui:** promote Facts into ui as definition-list card; restyle Pass ([e2aea6d](https://github.com/readur/readur/commit/e2aea6d8f8649d4edf7a10c8b9ea40efcea3f6d7))
* **ui:** promote label chip to ui as dot pill; Studio filter chips ([7b72301](https://github.com/readur/readur/commit/7b723012c2e5cd487154b4d2b6a5032f8383eb9a))
* **ui:** promote Segmented to ui; underline tabs ([8dac50a](https://github.com/readur/readur/commit/8dac50aba33d4554910f17ac0a5b707994e4b898))
* **ui:** promote SourceBadge to ui as hue icon tile ([e550d0a](https://github.com/readur/readur/commit/e550d0a89a6378847b1969b5736c0be63994d2ab))
* **ui:** React Aria links carry router state ([67f2d0f](https://github.com/readur/readur/commit/67f2d0f6c2a8a83f5ca6b0c409c875478eaf6e63))
* **ui:** resizable drawers open at half the window and grow to 85% ([f1fd2a0](https://github.com/readur/readur/commit/f1fd2a083c58ad223e8dec8f4df301b788305d8a))
* **ui:** resizable slide-over with a grab handle; panels animate out ([e39300e](https://github.com/readur/readur/commit/e39300e811b071fbac40012dbffe7e9391ce3d1b))
* **ui:** row errors live in a status tooltip, rows stay one line ([1d2dbb0](https://github.com/readur/readur/commit/1d2dbb0b600ec061a522e281e99b4e336d98cd18))
* **ui:** SlideOver fill layout, opening share, arrow-key opt-out ([3c91d07](https://github.com/readur/readur/commit/3c91d075faf423b61fd4028a1bb50b10fdfa4be2))
* **ui:** Spinner, Card and Avatar primitives ([b7eceae](https://github.com/readur/readur/commit/b7eceae7ee76dda388eee49b1e58db9ea2844a61))
* **ui:** Studio buttons, soft-square icon buttons, tooltip arrow ([5ff4d1e](https://github.com/readur/readur/commit/5ff4d1e22852a00063b697ee76957809070015a1))
* **ui:** Studio tokens, teal accent, Plus Jakarta Sans + JetBrains Mono ([c817985](https://github.com/readur/readur/commit/c817985f23e2396da9ef4aa84517a1656d33f291))
* **ui:** top-anchored dialog with icon tile, grouped menus ([05ab51a](https://github.com/readur/readur/commit/05ab51adb6a33736ecd1084614701a917ad3be75))


### Bug Fixes

* **deps:** resolve the font packages from the public npm registry ([18600bb](https://github.com/readur/readur/commit/18600bb6d202adb6456852f557c01df0f4fcb671))
* **dev:** stop cargo-watch restart loop in dev compose ([fda8c33](https://github.com/readur/readur/commit/fda8c3302c4d2641069a3867fda8c7b4cb92ecde))
* **document:** an older failed label save no longer restores labels on the list row ([bd5c74d](https://github.com/readur/readur/commit/bd5c74d27c6a6d2da51e1ccdb13bd70559bc42b5))
* **drawers:** a second close() before the URL changes no longer goes back twice ([3274f28](https://github.com/readur/readur/commit/3274f28d905932f4cc02a25a28dceb74bc710b3d))
* **library:** a slow OCR poll can't bring back older rows or undo a drawer change ([8a814e4](https://github.com/readur/readur/commit/8a814e45b10ef64ab1df81f73b86a5693f587d52))
* **library:** URL updates keep params that are not the Library's ([5f55b41](https://github.com/readur/readur/commit/5f55b41799e54d8577251bcc56559ea898960397))
* **mobile:** 16px text fields on touch screens so iOS stops zooming the page in ([be45f9c](https://github.com/readur/readur/commit/be45f9cd65be5f2139981057c90044fdd8ca02b5))
* **npm:** update dependency @types/node (22.20.4 → 22.20.5) ([#931](https://github.com/readur/readur/issues/931)) ([bc6500a](https://github.com/readur/readur/commit/bc6500a5435abbc58583df56ce7d53462fb525cf))
* **npm:** update dependency vite (7.3.6 → 7.3.7) ([#945](https://github.com/readur/readur/issues/945)) ([ca258e7](https://github.com/readur/readur/commit/ca258e752e36821687883454d5ee9e1a35cb0d1f))
* **rust:** update crate serde_json (1.0.151 → 1.0.152) ([#958](https://github.com/readur/readur/issues/958)) ([af2509f](https://github.com/readur/readur/commit/af2509fb162c8366d4655fb053dc0f40f4d931b4))
* **rust:** update crate tokio (1.53.1 → 1.53.2) ([#936](https://github.com/readur/readur/issues/936)) ([7c22d5e](https://github.com/readur/readur/commit/7c22d5e116162ce47265e2026feb4e6358f72259))
* **rust:** update crate tokio-util (0.7.19 → 0.7.20) ([#955](https://github.com/readur/readur/issues/955)) ([c7ceda8](https://github.com/readur/readur/commit/c7ceda846eb4568586cea9983e99cb51a946befc))
* **seed:** forward ADMIN_* through compose and treat a blank ADMIN_PASSWORD as unset ([27226c0](https://github.com/readur/readur/commit/27226c00010b3187d89b35aa808d2a173584a076))
* **shell:** phone tab bar says "Search", icons line up and labels stay on one line ([6f65163](https://github.com/readur/readur/commit/6f65163c448bea7c68ce3cd6cf5e225a25f4edfd))
* **sources:** watched folders wrap one per line in the details panel; PassCell wrap ([177da53](https://github.com/readur/readur/commit/177da53471e776536479ce803f46cc0411acd4f2))
* **ui:** drawer preview leaves room for the tabs at 900px; SlideOver grab handle hidden on phones ([4ea9622](https://github.com/readur/readur/commit/4ea962251e75131a4c0a573336633a20023537d9))
* **ui:** IconButton passes the button to the caller's ref as well as its own ([5309760](https://github.com/readur/readur/commit/530976027a9150b0c949984077b4e5dd16e368c2))
* **ui:** New/Changed tag sits in the card's status row; the ring outlines the card ([3cfb9a6](https://github.com/readur/readur/commit/3cfb9a6c29a300ef9cdef91ed21eccab349d2bf8))
* **ui:** previews fill the card well, status words keep descenders, 4-up Just arrived ([5dd57e7](https://github.com/readur/readur/commit/5dd57e76fec6dbb34307f28a09f9b460a6e3b933))
* **ui:** review fixes — backend source types, dialog fits viewport, invalid label colours, quiet palette preview ([02a4592](https://github.com/readur/readur/commit/02a4592a8f47e36a4f7d954456804b70a9b2d045))
* **ui:** scope Progress tones and source slots to their own components ([871a24f](https://github.com/readur/readur/commit/871a24f75f12b1079f618abcf38ea4635ca23325))
* **ui:** system label detachment and deletion ([2e0d187](https://github.com/readur/readur/commit/2e0d187c225bdf77149a810311463ce2fa0d1564))
* **ui:** tab rows no longer show a stray vertical scrollbar; hairline is an inset line ([9adf79f](https://github.com/readur/readur/commit/9adf79ffac608aca68c26a95ee36eba255f134e9))
* **ui:** toasts pause while hovered, drain bar included ([208e0c9](https://github.com/readur/readur/commit/208e0c9cd4ad5afaaa3f1da3084b738f5223e034))


### Code Refactoring

* **document:** lift comment loading into useComments (count, polling only when shown) ([bb1948e](https://github.com/readur/readur/commit/bb1948eb4cca0c9d7bce2f05f767409ad33af7f8))


### Documentation

* add Studio design system v3 spec ([eea3db5](https://github.com/readur/readur/commit/eea3db5f4dd9f7f3d828bb1229d2be9009603c8e))
* labels are per account; describe the five default labels ([09dfb43](https://github.com/readur/readur/commit/09dfb43dca6c17181a8a68f9eff25c0f2cbbc936))
* new screenshots of the redesigned UI and a gallery that matches them ([6b59fd0](https://github.com/readur/readur/commit/6b59fd09767688877d4bdb91d9b103b3d627a1bc))
* readur-design-system skill and Studio DESIGN.md ([579656a](https://github.com/readur/readur/commit/579656a4e529876e840c120795a4379e8387763c))
* retake the phone screenshots with the aligned tab bar ([310421e](https://github.com/readur/readur/commit/310421e41e15df065fe806664874fc68d7f8b2d8))
* **skill:** ButtonLink, SourceTile, StatusMark label, resizable SlideOver ([af3ecd9](https://github.com/readur/readur/commit/af3ecd94565054905ae82f03ddcc1ee512a5db5b))
* **skill:** drawers live in the URL; the document drawer; SlideOver fill/share/own-arrows ([5eb4a1b](https://github.com/readur/readur/commit/5eb4a1bf7e63a2f562ef07d12c108c6bec01b85f))
* **skill:** row errors via StatusMark reason, disabledReason, progress toasts ([cb0f78d](https://github.com/readur/readur/commit/cb0f78d0cc5f60cd8770fd3453cd506edf4d48c2))


### Tests

* **e2e:** the document drawer replaces the document page ([c19a0e7](https://github.com/readur/readur/commit/c19a0e7b800e0ef62d379bda1e12e40daec987a4))
* **e2e:** theme colours from the current palette; folders live under Watched folders ([ce2425f](https://github.com/readur/readur/commit/ce2425f541870e2bdefc8f33d0489a841200a878))
* **intake:** wait for the connection drawer to close after a delete (history step) ([022a1a1](https://github.com/readur/readur/commit/022a1a13d352621e4e5acc7f7ad18b77565e5735))
* **ui:** enforce tokens-only primitive stylesheets ([3aeccfa](https://github.com/readur/readur/commit/3aeccfac7f0d220442804bd65a2c86b4a45db36d))


### Continuous Integration

* auto-merge 0.x Renovate updates once CI passes ([16a3271](https://github.com/readur/readur/commit/16a32718fac62eeb070b40d2fe105b7632f4f54f))
* keep major Renovate updates as open PRs ([b1bd8b1](https://github.com/readur/readur/commit/b1bd8b19a05a475c52ecc6d47408c85cee6ac2bf))
* manage versions and releases with release-please ([1356622](https://github.com/readur/readur/commit/13566226e17638d875a9c500f87ed0a12b254d4c))
* manage versions and releases with release-please ([9207179](https://github.com/readur/readur/commit/920717997ebdfa9103dc8440fc6db4c39a9f9be8))
* pass the tag to the version check through an env var ([7927fd8](https://github.com/readur/readur/commit/7927fd87653876fa03c10d60aee1f0ae725d0920))
* stop Renovate rebases and superseded runs from flooding the queue ([04c982d](https://github.com/readur/readur/commit/04c982d55df4014c2238c0deb4876d8d7efce332))
* stop Renovate rebases and superseded runs from flooding the queue ([a509a23](https://github.com/readur/readur/commit/a509a2370cb4e88c6902f4bf0cfaac281e1ef716))


### Miscellaneous Chores

* **github-action:** update action dtolnay/rust-toolchain (6bed076 → 89b1218) ([#922](https://github.com/readur/readur/issues/922)) ([e548430](https://github.com/readur/readur/commit/e5484301b9700bcb73e3e34b90ce66a891d410ed))
* **github-action:** update action dtolnay/rust-toolchain (89b1218 → 686976e) ([#944](https://github.com/readur/readur/issues/944)) ([a13cde4](https://github.com/readur/readur/commit/a13cde4f69a0605cee9852fdc5dcf10aa6fc972d))
* **github-action:** update action github/codeql-action (v4.38.2 → v4.38.3) ([#953](https://github.com/readur/readur/issues/953)) ([1021062](https://github.com/readur/readur/commit/1021062db7183b08d2b43bc90054857360351d02))
* **github-action:** update action int128/diff-action (v2.40.0 → v2.41.0) ([#933](https://github.com/readur/readur/issues/933)) ([0492bd6](https://github.com/readur/readur/commit/0492bd62c295db52b186bd1660ab7235791eba93))
* **github-action:** update action jdx/mise-action (v5.0.0 → v5.0.1) ([#927](https://github.com/readur/readur/issues/927)) ([195fe64](https://github.com/readur/readur/commit/195fe649ffd7fc55324878b699d375d45025d3c5))
* **github-action:** update action jdx/mise-action (v5.0.1 → v5.1.0) ([#937](https://github.com/readur/readur/issues/937)) ([e488366](https://github.com/readur/readur/commit/e4883663a0ef4940cdca2f1c9fc578c039834ca4))
* **github-action:** update action jdx/mise-action (v5.1.0 → v5.1.1) ([#938](https://github.com/readur/readur/issues/938)) ([a4e619a](https://github.com/readur/readur/commit/a4e619a9cad271f973c4d9d00ed6b85f6cbda55b))
* ignore Superpowers planning docs (docs/superpowers/) ([b6b5871](https://github.com/readur/readur/commit/b6b58711e0159399017b8bc678d8ec42a6baddd9))
* **mise:** update tool lefthook (2.1.14 → 2.1.15) ([#924](https://github.com/readur/readur/issues/924)) ([f33f2fb](https://github.com/readur/readur/commit/f33f2fbb728587d5cb53ae5c5348f435df9ebd28))
* **mise:** update tool lefthook (2.1.15 → 2.1.16) ([#929](https://github.com/readur/readur/issues/929)) ([48caac0](https://github.com/readur/readur/commit/48caac06c14bdda052cc8ef062e8ea86f7227c53))
* **mise:** update tool lefthook (2.1.16 → 2.1.17) ([#942](https://github.com/readur/readur/issues/942)) ([2983477](https://github.com/readur/readur/commit/2983477864da5740e037909e1419b9f3d09defa8))
* **mise:** update tool lefthook (2.1.17 → 2.2.0) ([#948](https://github.com/readur/readur/issues/948)) ([e234882](https://github.com/readur/readur/commit/e2348821926b6d745af9f69bc5bdf23fa4e0393d))
* **mise:** update tool lefthook (2.2.0 → 2.2.1) ([#952](https://github.com/readur/readur/issues/952)) ([8a27786](https://github.com/readur/readur/commit/8a27786976c9b02422d97ad021e4d9642d329747))
* **mise:** update tool oxfmt (0.71.0 → 0.72.0) ([#943](https://github.com/readur/readur/issues/943)) ([661fad8](https://github.com/readur/readur/commit/661fad8af761494ec4e9a9b2a9a2857e6f189048))
* **mise:** update tool rust (1.98.1 → 1.99.0) ([#928](https://github.com/readur/readur/issues/928)) ([c54a49c](https://github.com/readur/readur/commit/c54a49c47110def0ff888651b137b46ede16d97d))
