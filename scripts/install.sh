
#!/usr/bin/env bash
set -euo pipefail

REPO="GorkiyChocolate/DevInit-CLI"
BIN_NAME="devinit"
ARCHIVE="devinit-linux-x64.tar.gz"

fail() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

info() {
  printf '==> %s\n' "$*"
}

in_path() {
  case ":${PATH}:" in
    *":$1:"*) return 0 ;;
    *) return 1 ;;
  esac
}

for cmd in curl tar sha256sum mktemp uname install mv grep getconf; do
  command -v "$cmd" >/dev/null 2>&1 ||
    fail "Missing required command: $cmd"
done

[[ "$(uname -s)" == "Linux" ]] ||
  fail "This installer supports Linux only."

case "$(uname -m)" in
  x86_64|amd64) ;;
  *) fail "Unsupported architecture. Only x86_64 is supported." ;;
esac

getconf GNU_LIBC_VERSION >/dev/null 2>&1 ||
  fail "This release requires a glibc-based Linux distribution."

# Prefer a user installation without administrator privileges.
# If its directory is not in PATH, use the standard system path.
if in_path "${HOME}/.local/bin"; then
  INSTALL_DIR="${HOME}/.local/bin"
  USE_SUDO=false
elif in_path "/usr/local/bin"; then
  INSTALL_DIR="/usr/local/bin"
  USE_SUDO=true
else
  fail "Neither ~/.local/bin nor /usr/local/bin is in PATH. \
Cannot guarantee that the devinit command will be accessible."
fi

DEST="${INSTALL_DIR}/${BIN_NAME}"

# Detect another installation that would take priority in PATH.
if command -v devinit >/dev/null 2>&1; then
  CURRENT="$(command -v devinit)"

  if [[ "$CURRENT" != "$DEST" ]]; then
    fail "Another DevInit executable takes priority: $CURRENT. \
Refusing to create a conflicting installation."
  fi
fi

# Never replace symbolic links or unrelated executables.
if [[ -L "$DEST" || -d "$DEST" ]]; then
  fail "Installation target is not a regular file: $DEST"
fi

if [[ -e "$DEST" ]]; then
  CURRENT_VERSION="$("$DEST" --version 2>/dev/null || true)"

  if [[ ! "$CURRENT_VERSION" =~ ^devinit[[:space:]][0-9] ]]; then
    fail "Existing file is not recognized as DevInit: $DEST"
  fi

  info "Existing installation: $CURRENT_VERSION"
fi

if [[ -n "${DEVINIT_VERSION:-}" ]]; then
  VERSION="$DEVINIT_VERSION"

  [[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] ||
    fail "Invalid version: $VERSION"

  BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
  EXPECTED_VERSION="${VERSION#v}"
else
  BASE_URL="https://github.com/${REPO}/releases/latest/download"
  EXPECTED_VERSION=""
fi

TMP_DIR="$(mktemp -d)"
STAGED=""

cleanup() {
  if [[ -n "$STAGED" ]]; then
    if [[ "$USE_SUDO" == true ]]; then
      sudo rm -f -- "$STAGED" || true
    else
      rm -f -- "$STAGED"
    fi
  fi

  rm -rf -- "$TMP_DIR"
}

trap cleanup EXIT

info "Downloading DevInit..."

curl -fsSL --retry 3 \
  "${BASE_URL}/${ARCHIVE}" \
  -o "${TMP_DIR}/${ARCHIVE}"

curl -fsSL --retry 3 \
  "${BASE_URL}/${ARCHIVE}.sha256" \
  -o "${TMP_DIR}/${ARCHIVE}.sha256"

info "Checking SHA-256..."

(
  cd "$TMP_DIR"
  sha256sum -c --status "${ARCHIVE}.sha256"
) || fail "SHA-256 verification failed."

info "Extracting DevInit..."

# Extract the executable to a known path rather than
# unpacking arbitrary archive paths into the filesystem.
tar -xOzf "${TMP_DIR}/${ARCHIVE}" devinit \
  > "${TMP_DIR}/devinit" ||
  fail "Cannot extract DevInit executable."

chmod 700 "${TMP_DIR}/devinit"

info "Verifying executable..."

DOWNLOADED_VERSION="$(
  "${TMP_DIR}/devinit" --version 2>/dev/null
)" || fail "Downloaded binary cannot execute."

[[ "$DOWNLOADED_VERSION" =~ ^devinit[[:space:]][0-9] ]] ||
  fail "Downloaded executable is not recognized as DevInit."

if [[ -n "$EXPECTED_VERSION" ]]; then
  [[ "$DOWNLOADED_VERSION" == "devinit $EXPECTED_VERSION" ]] ||
    fail "Version mismatch: expected $EXPECTED_VERSION, \
got $DOWNLOADED_VERSION"
fi

info "Downloaded: $DOWNLOADED_VERSION"

if [[ "$USE_SUDO" == false ]]; then
  mkdir -p -- "$INSTALL_DIR"

  [[ -w "$INSTALL_DIR" ]] ||
    fail "Cannot write to $INSTALL_DIR"

  STAGED="$(mktemp "${INSTALL_DIR}/.devinit-install.XXXXXXXX")"

  install -m 0755 "${TMP_DIR}/devinit" "$STAGED"

  "$STAGED" --version >/dev/null ||
    fail "Staged executable failed verification."

  mv -fT -- "$STAGED" "$DEST"
else
  command -v sudo >/dev/null 2>&1 ||
    fail "sudo is required for system installation."

  sudo test -d "$INSTALL_DIR" ||
    fail "System installation directory does not exist."

  STAGED="$(
    sudo mktemp "${INSTALL_DIR}/.devinit-install.XXXXXXXX"
  )"

  sudo install -m 0755 "${TMP_DIR}/devinit" "$STAGED"

  "$STAGED" --version >/dev/null ||
    fail "Staged executable failed verification."

  sudo mv -fT -- "$STAGED" "$DEST"
fi

STAGED=""

info "DevInit installed successfully."

"$DEST" --version

info "Installation path: $DEST"
