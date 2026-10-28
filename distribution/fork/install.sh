#!/bin/sh
# Aira fork: derived from the Herdr installer with fork-specific downloads.
set -eu

BIN="herdr-aira"
MANIFEST_URL="https://github.com/three4c/herdr/releases/latest/download/fork-latest.json"
INSTALL_DIR="${HERDR_INSTALL_DIR:-$HOME/.local/bin}"

main() {
    log "Aira installer (three4c/herdr)"

    # detect platform
    OS="$(uname -s)"
    case "$OS" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)      err "unsupported OS: $OS" ;;
    esac

    if [ "$OS" = "Linux" ] && [ "$(uname -o 2>/dev/null || true)" = "Android" ]; then
        err "Android/Termux is not currently supported by Herdr release binaries. SSH to a supported host instead: https://github.com/three4c/herdr"
    fi

    ARCH="$(uname -m)"
    case "$ARCH" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        *)              err "unsupported architecture: $ARCH" ;;
    esac

    log "detected ${os}/${arch}"

    # check dependencies
    need curl
    need awk

    # use the same manifest as `herdr-aira update` so installs and updates agree
    # on the public latest release.
    TARGET="${os}-${arch}"
    log "fetching latest release manifest..."
    MANIFEST="$(curl -fsSL --retry 3 --connect-timeout 10 --max-time 20 "$MANIFEST_URL")" \
        || err "can't reach ${MANIFEST_URL}. No published fork release may be available yet."
    URL="$(printf '%s\n' "$MANIFEST" | awk -v target="\"${TARGET}\"" '
        /^[[:space:]]*"assets"[[:space:]]*:/ { in_assets = 1; next }
        in_assets && /^[[:space:]]*}/ { exit }
        in_assets && index($0, target) {
            sub(/^.*:[[:space:]]*"/, "")
            sub(/".*$/, "")
            print
            exit
        }
    ')"
    SHA256="$(printf '%s\n' "$MANIFEST" | awk -v target="\"${TARGET}\"" '
        /^[[:space:]]*"sha256"[[:space:]]*:/ { in_sha256 = 1; next }
        in_sha256 && /^[[:space:]]*}/ { exit }
        in_sha256 && index($0, target) {
            sub(/^.*:[[:space:]]*"/, "")
            sub(/".*$/, "")
            print
            exit
        }
    ')"
    VERSION="$(printf '%s\n' "$MANIFEST" | awk -F '"' '/^[[:space:]]*"version"[[:space:]]*:/ { print $4; exit }')"

    DISTRIBUTION="$(printf '%s\n' "$MANIFEST" | awk -F '"' '/^[[:space:]]*"distribution"[[:space:]]*:/ { print $4; exit }')"
    RELEASE="$(printf '%s\n' "$MANIFEST" | awk -F '"' '/^[[:space:]]*"release"[[:space:]]*:/ { print $4; exit }')"
    BASE_VERSION="$(printf '%s\n' "$MANIFEST" | awk -F '"' '/^[[:space:]]*"base_version"[[:space:]]*:/ { print $4; exit }')"
    [ "$DISTRIBUTION" = "three4c/herdr" ] || err "invalid fork distribution"
    valid_version "$RELEASE" || err "invalid fork release version"
    [ "$RELEASE" != "0.0.0" ] || err "local builds cannot be installed as releases"
    valid_version "$BASE_VERSION" || err "invalid fork base version"
    [ "$VERSION" = "${BASE_VERSION}-aira.${RELEASE}" ] || err "invalid fork version identity"
    EXPECTED_URL="https://github.com/three4c/herdr/releases/download/aira-v${RELEASE}/herdr-${TARGET}"
    [ "$URL" = "$EXPECTED_URL" ] || err "binary URL must belong to the matching three4c/herdr release"

    if [ -z "$URL" ]; then
        err "release manifest does not include a binary for ${TARGET}"
    fi
    if [ "${#SHA256}" -ne 64 ]; then
        err "release manifest does not include a valid SHA-256 checksum for ${TARGET}"
    fi
    if ! printf '%s\n' "$SHA256" | awk '/[^0-9A-Fa-f]/ { exit 1 }'; then
        err "release manifest does not include a valid SHA-256 checksum for ${TARGET}"
    fi
    SHA256="$(printf '%s\n' "$SHA256" | awk '{ print tolower($0) }')"

    if command -v sha256sum >/dev/null 2>&1; then
        SHA256_TOOL="sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        SHA256_TOOL="shasum"
    elif command -v openssl >/dev/null 2>&1; then
        SHA256_TOOL="openssl"
    else
        err "SHA-256 verification requires sha256sum, shasum, or openssl"
    fi

    if [ -n "$VERSION" ]; then
        log "downloading v${VERSION}..."
    else
        log "downloading latest release..."
    fi
    TMP="$(mktemp -d)"
    STAGE=''
    trap 'rm -rf "$TMP"; if [ -n "$STAGE" ]; then rm -rf "$STAGE"; fi' 0
    trap 'exit 1' HUP INT TERM

    if ! curl -fsSL --retry 3 --connect-timeout 10 --max-time 120 "$URL" -o "${TMP}/${BIN}"; then
        err "download failed from ${URL}"
    fi

    case "$SHA256_TOOL" in
        sha256sum) ACTUAL_SHA256="$(sha256sum < "${TMP}/${BIN}" | awk '{ print $1 }')" ;;
        shasum)    ACTUAL_SHA256="$(shasum -a 256 < "${TMP}/${BIN}" | awk '{ print $1 }')" ;;
        openssl)   ACTUAL_SHA256="$(openssl dgst -sha256 < "${TMP}/${BIN}" | awk '{ print $NF }')" ;;
    esac
    if [ "$ACTUAL_SHA256" != "$SHA256" ]; then
        err "downloaded Herdr checksum did not match"
    fi

    # install
    mkdir -p "$INSTALL_DIR"
    [ ! -d "${INSTALL_DIR}/${BIN}" ] || err "install destination is a directory: ${INSTALL_DIR}/${BIN}"
    # Stage on the destination filesystem, then atomically replace the executable.
    STAGE="$(mktemp -d "${INSTALL_DIR}/.herdr-install.XXXXXX")"
    cp "${TMP}/${BIN}" "${STAGE}/${BIN}"
    chmod +x "${STAGE}/${BIN}"
    mv -f "${STAGE}/${BIN}" "${INSTALL_DIR}/${BIN}"

    log "installed ${BIN} to ${INSTALL_DIR}/${BIN}"

    # check PATH
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo ""
            warn "${INSTALL_DIR} is not in your PATH"
            echo "  add it to your shell config:"
            echo ""
            echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
            echo ""
            ;;
    esac

    # Report PATH shadowing without executing or replacing another installation.
    RESOLVED="$(command -v "$BIN" 2>/dev/null || true)"
    if [ -n "$RESOLVED" ] && [ "$RESOLVED" != "${INSTALL_DIR}/${BIN}" ]; then
        warn "PATH selects ${RESOLVED}; put ${INSTALL_DIR} first to use this fork"
    fi
    if [ "$RESOLVED" = "${INSTALL_DIR}/${BIN}" ]; then
        echo ""
        log "ready. run 'herdr-aira' to get started."
    fi

    echo ""
}

log()  { printf '  \033[32m>\033[0m %s\n' "$1"; }
warn() { printf '  \033[33m!\033[0m %s\n' "$1"; }
err()  { printf '  \033[31m✗\033[0m %s\n' "$1" >&2; exit 1; }

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "requires '$1' — install it first, or download a binary manually from https://github.com/three4c/herdr/releases"
    fi
}

valid_version() {
    printf '%s\n' "$1" | awk -F . '
        /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/ {
            if ($1 <= 4294967295 && $2 <= 4294967295 && $3 <= 4294967295) valid = 1
        }
        END { exit !valid }
    '
}

main "$@"
