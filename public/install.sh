#!/bin/sh
set -e

REPO="ergofobe/orai"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"
GITHUB_URL="https://github.com/${REPO}"
INSTALL_DIR="${HOME}/.local/bin"

BOLD='\033[1m'
RED='\033[31m'
GREEN='\033[32m'
YELLOW='\033[33m'
CYAN='\033[36m'
RESET='\033[0m'

info()  { printf "${CYAN}${BOLD}info${RESET} %s\n" "$1"; }
warn()  { printf "${YELLOW}${BOLD}warn${RESET} %s\n" "$1"; }
error() { printf "${RED}${BOLD}error${RESET} %s\n" "$1"; }

FORCE=0
USE_GLIBC=0
DRY_RUN=0

for arg in "$@"; do
    case "$arg" in
        --glibc) USE_GLIBC=1 ;;
        --force) FORCE=1 ;;
        --dry-run) DRY_RUN=1 ;;
        -h|--help)
            echo "Usage: install.sh [options]"
            echo ""
            echo "Options:"
            echo "  --glibc     Use glibc binary instead of musl (Linux only)"
            echo "  --force     Install even if already up to date"
            echo "  --dry-run   Show what would be installed, don't actually install"
            echo "  -h, --help  Show this help message"
            exit 0
            ;;
        *)
            error "Unknown option: $arg"
            exit 1
            ;;
    esac
done

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

IS_TERMUX=0
if [ -n "$TERMUX_VERSION" ] || [ -d "/data/data/com.termux" ]; then
    IS_TERMUX=1
fi

case "$OS" in
    darwin)
        PLATFORM="macOS"
        ;;
    linux)
        if [ "$IS_TERMUX" = "1" ]; then
            PLATFORM="Android"
        else
            PLATFORM="Linux"
        fi
        ;;
    *)
        error "Unsupported operating system: $OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH_NAME="x86_64"
        ;;
    aarch64|arm64)
        ARCH_NAME="aarch64"
        ;;
    *)
        error "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

info "Installing orai for ${PLATFORM} (${ARCH_NAME})..."

if [ "$PLATFORM" = "macOS" ]; then
    if command -v brew >/dev/null 2>&1; then
        info "Homebrew detected. Installing via Homebrew..."
        if [ "$DRY_RUN" = "1" ]; then
            info "Would run: brew install ergofobe/orai/orai"
            exit 0
        fi
        brew install ergofobe/orai/orai
        info "orai installed via Homebrew."
        orai --version
        exit 0
    else
        warn "Homebrew not found."
        printf "${BOLD}Install Homebrew first? [Y/n]${RESET} "
        read -r install_brew
        case "$install_brew" in
            n*|N*)
                warn "Falling back to direct binary download..."
                ;;
            *)
                info "Installing Homebrew..."
                /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
                if command -v brew >/dev/null 2>&1; then
                    brew install ergofobe/orai/orai
                    info "orai installed via Homebrew."
                    orai --version
                    exit 0
                else
                    error "Homebrew installation failed. Falling back to direct binary download."
                fi
                ;;
        esac
    fi
fi

if [ "$PLATFORM" = "Android" ]; then
    TARGET="${ARCH_NAME}-linux-android"
elif [ "$PLATFORM" = "Linux" ]; then
    if [ "$USE_GLIBC" = "1" ]; then
        TARGET="${ARCH_NAME}-unknown-linux-gnu"
    else
        TARGET="${ARCH_NAME}-unknown-linux-musl"
    fi
else
    TARGET="${ARCH_NAME}-apple-darwin"
fi

info "Fetching latest release information..."
RELEASE_URL="https://api.github.com/repos/${REPO}/releases/latest"
TAG=$(curl -fsSL "$RELEASE_URL" 2>/dev/null | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name":\s*"([^"]+)".*/\1/')

if [ -z "$TAG" ]; then
    error "Failed to fetch latest release. Check your internet connection."
    exit 1
fi

VERSION="${TAG#v}"
info "Latest version: ${VERSION}"

if command -v orai >/dev/null 2>&1; then
    INSTALLED=$(orai --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
    if [ -n "$INSTALLED" ]; then
        if [ "$FORCE" = "0" ]; then
            INSTALLED_MAJOR=$(echo "$INSTALLED" | cut -d. -f1)
            INSTALLED_MINOR=$(echo "$INSTALLED" | cut -d. -f2)
            INSTALLED_PATCH=$(echo "$INSTALLED" | cut -d. -f3)
            LATEST_MAJOR=$(echo "$VERSION" | cut -d. -f1)
            LATEST_MINOR=$(echo "$VERSION" | cut -d. -f2)
            LATEST_PATCH=$(echo "$VERSION" | cut -d. -f3)

            if [ "$INSTALLED_MAJOR" -eq "$LATEST_MAJOR" ] && \
               [ "$INSTALLED_MINOR" -eq "$LATEST_MINOR" ] && \
               [ "$INSTALLED_PATCH" -eq "$LATEST_PATCH" ]; then
                info "orai v${VERSION} is already installed and up to date."
                exit 0
            fi

            if [ "$INSTALLED_MAJOR" -gt "$LATEST_MAJOR" ] || \
               ([ "$INSTALLED_MAJOR" -eq "$LATEST_MAJOR" ] && [ "$INSTALLED_MINOR" -gt "$LATEST_MINOR" ]) || \
               ([ "$INSTALLED_MAJOR" -eq "$LATEST_MAJOR" ] && [ "$INSTALLED_MINOR" -eq "$LATEST_MINOR" ] && [ "$INSTALLED_PATCH" -gt "$LATEST_PATCH" ]); then
                warn "orai v${INSTALLED} is newer than the latest release v${VERSION}. Skipping."
                exit 0
            fi

            printf "${BOLD}orai v${INSTALLED} -> v${VERSION} available. Update? [Y/n]${RESET} "
            read -r confirm
            case "$confirm" in
                n*|N*)
                    info "Update skipped."
                    exit 0
                    ;;
            esac
        fi
    fi
fi

ARCHIVE="orai-${TARGET}.tar.gz"
DOWNLOAD_URL="${GITHUB_URL}/releases/download/${TAG}/${ARCHIVE}"

info "Downloading orai v${VERSION} for ${TARGET}..."
if [ "$DRY_RUN" = "1" ]; then
    info "Would download: ${DOWNLOAD_URL}"
    info "Would install to: ${INSTALL_DIR}/orai"
    exit 0
fi

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL "$DOWNLOAD_URL" -o "${TMPDIR}/${ARCHIVE}" || {
    error "Failed to download ${ARCHIVE}"
    exit 1
}

tar -xzf "${TMPDIR}/${ARCHIVE}" -C "${TMPDIR}" || {
    error "Failed to extract archive"
    exit 1
}

if [ ! -f "${TMPDIR}/orai" ]; then
    error "Binary not found in archive"
    exit 1
fi

mkdir -p "$INSTALL_DIR"
chmod +x "${TMPDIR}/orai"
mv "${TMPDIR}/orai" "${INSTALL_DIR}/orai"

SHELL_RC=""
case "$SHELL" in
    */zsh) SHELL_RC="${HOME}/.zshrc" ;;
    */bash) SHELL_RC="${HOME}/.bashrc" ;;
    *) SHELL_RC="${HOME}/.profile" ;;
esac

PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'
if ! echo ":$PATH:" | grep -q ":${INSTALL_DIR}:"; then
    if ! grep -q "$PATH_LINE" "$SHELL_RC" 2>/dev/null; then
        echo "" >> "$SHELL_RC"
        echo "$PATH_LINE" >> "$SHELL_RC"
        info "Added ${INSTALL_DIR} to PATH in ${SHELL_RC}"
        info "Run: source ${SHELL_RC}  # or start a new shell"
    fi
fi

info "${GREEN}orai v${VERSION} installed successfully!${RESET}"
"${INSTALL_DIR}/orai" --version