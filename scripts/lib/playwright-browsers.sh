# shellcheck shell=bash
# Shared by run-tests.sh and scripts/run-e2e-local.sh: installs what the Playwright suite in
# frontend/ needs, so the two runners cannot drift apart. Source it, then call
# install_playwright_browsers from the frontend directory.
#
# PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH (read by frontend/playwright.config.ts) selects a system
# Chromium (NixOS, for example); then only Playwright's ffmpeg is installed. Otherwise the
# Playwright Chromium is downloaded, with its system packages (`--with-deps`) on CI and on
# Debian-like hosts, where Playwright knows how to install them.
#
# Any failure is returned to the caller, with a message on stderr.

# Whether `npx playwright install --with-deps` can install system packages here.
playwright_can_install_deps() {
    [ -n "${CI:-}" ] && return 0
    [ -r /etc/os-release ] || return 1
    # ID and ID_LIKE name the distribution family ("debian", "ubuntu").
    grep -Eq '^ID(_LIKE)?="?[^"]*\b(debian|ubuntu)\b' /etc/os-release
}

install_playwright_browsers() {
    if [ -n "${PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH:-}" ]; then
        if [ ! -x "$PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH" ]; then
            echo "PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH ($PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH) is not an executable" >&2
            return 1
        fi
        echo "Using the system Chromium at $PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH" >&2
        # playwright.config.ts records video on failure, which needs Playwright's own ffmpeg;
        # without it pages fail to open. The system Chromium brings its own libraries, so no
        # --with-deps.
        if ! npx playwright install ffmpeg >&2; then
            echo "Could not install Playwright's ffmpeg (point PLAYWRIGHT_BROWSERS_PATH at a writable directory)" >&2
            return 1
        fi
        return 0
    fi

    local deps=()
    if playwright_can_install_deps; then
        deps=(--with-deps)
    fi
    echo "Installing the Playwright Chromium browser ${deps[*]:-}..." >&2
    if ! npx playwright install ${deps[@]+"${deps[@]}"} chromium >&2; then
        echo "Could not install the Playwright Chromium browser" >&2
        return 1
    fi
}
