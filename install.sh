#!/usr/bin/env bash
set -euo pipefail
trap 'tput cnorm 2>/dev/null; die "Command failed at line $LINENO: $BASH_COMMAND"' ERR
trap 'tput cnorm 2>/dev/null' EXIT

# --- config ---
REPO="MintyDoggo/miri"
INSTALL_DIR="$HOME/.local/bin"
SERVICE_DIR="$HOME/.config/systemd/user"
WANTS_DIR="$SERVICE_DIR/niri.service.wants"
BINARY_ASSET="miri-x86_64"
SERVICE_ASSET="miri.service"
CONFIG_ASSET="config.toml"
CONFIG_DIR="$HOME/.config/miri"

# --- helpers ---
RED='\033[31m'
GREEN='\033[32m'
YELLOW='\033[33m'
BLUE='\033[34m'
MAGENTA='\033[35m'
CYAN='\033[36m'
WHITE='\033[37m'
BOLD='\033[1m'
GREY='\033[0;90m'
RESET='\033[0m'

TEXT=${GREY}

info() { echo -e    "${BLUE}[Miri Info]:${GREY} $*${RESET}"; }
success() { echo -e "${GREEN}[Miri Success]:${GREY} $*${RESET}"; }
die()  { echo -e    "${RED}[Miri Error]:${GREY} $*${RESET}" >&2; exit 1; }

# --- menu ---
# Usage: select_menu "Prompt text" "Option 1" "Option 2" ...
# Result is stored in MENU_RESULT
select_menu() {
  local prompt="$1"
  shift
  local options=("$@")
  local selected=0
  local count=${#options[@]}
  local key key2

  tput civis
  echo -e "$prompt"

  # Initial draw
  for i in "${!options[@]}"; do
    if [[ $i -eq $selected ]]; then
      echo -e "  ${CYAN}>${RESET} ${BOLD}${options[$i]}${RESET}"
    else
      echo -e "    ${GREY}${options[$i]}${RESET}"
    fi
  done

  while true; do
    tput cuu "$count"

    for i in "${!options[@]}"; do
      tput el
      if [[ $i -eq $selected ]]; then
        echo -e "  ${CYAN}>${RESET} ${BOLD}${options[$i]}${RESET}"
      else
        echo -e "    ${GREY}${options[$i]}${RESET}"
      fi
    done

    IFS= read -rsn1 key
    if [[ $key == $'\x1b' ]]; then
      read -rsn2 -t 0.1 key2 || true
      case "$key2" in
        '[A') selected=$(( (selected - 1 + count) % count )) ;;
        '[B') selected=$(( (selected + 1) % count )) ;;
      esac
    elif [[ $key == '' || $key == $'\n' ]]; then
      break
    fi
  done

  tput cnorm
  MENU_RESULT="${options[$selected]}"
}

# --- checks ---
command -v curl >/dev/null || die "curl is required"
command -v systemctl >/dev/null || die "systemctl is required"
systemctl --user show-environment >/dev/null 2>&1 || die "systemd user session not available"

# ascii art greeter (a mess)
echo ""
echo -e "${BOLD}Miri${RESET} ${TEXT}(modal niri)${RESET}"
echo -e "${RESET}╔═══════-□×╗${TEXT}╔═════-□×╗    ${RESET}╔══════-□×${RESET}╗${TEXT}╔══════-□×╗"
echo -e "${RESET}║${TEXT}⠿⠿⠯⠥${RESET}      ║${TEXT}╚════════╝    ${RESET}║${TEXT}>_ ${CYAN}${BOLD}miri${RESET}  ║${GREY}║>_       ║"
echo -e "${RESET}║${TEXT}⠿⠿⠶⠶⠶⠶⠶⠦⠤${RESET} ║${TEXT}╔═════-□×╗    ${RESET}╚═════════╝${GREY}╚═════════╝"
echo -e "${RESET}║${TEXT}⠿⠿⠿⠯⠭⠭⠉⠉${RESET}  ║${TEXT}╚════════╝    ╔══════-□×╗${GREY}╔══════-□×╗"
echo -e "${RESET}║${TEXT}>_ ${CYAN}${BOLD}miri${RESET}   ║${TEXT}╔═════-□×╗    ${GREY}║>_       ║║>_       ║"
echo -e "${RESET}╚══════════╝${TEXT}╚════════╝    ╚═════════╝╚═════════╝${RESET}"
echo ""
# --- main flow ---
select_menu "${BOLD}What would you like to do?${RESET}" \
  "Install miri" \
  "Uninstall miri"
ACTION="$MENU_RESULT"
echo ""

if [[ "$ACTION" == "Install miri" ]]; then

  select_menu "${BOLD}Set up the systemd user service?${RESET}" \
    "Yes (recommended)" \
    "No"
  SETUP_SERVICE="$MENU_RESULT"
  echo ""

  # --- fetch release manifest ---
  info "Fetching latest release info from $REPO"
  RELEASE_JSON=$(curl -sf "https://api.github.com/repos/$REPO/releases/latest") \
    || die "Failed to fetch release info. Check if $REPO has published releases"

  get_asset_url() {
    local asset_name="$1"
    local url
    url=$(echo "$RELEASE_JSON" \
      | grep -o '"browser_download_url": *"[^"]*'"$asset_name"'[^"]*"' \
      | cut -d'"' -f4)
    [[ -n "$url" ]] || die "Asset '$asset_name' not found in latest release"
    echo "$url"
  }

  # --- install binary ---
  BINARY_URL=$(get_asset_url "$BINARY_ASSET")
  mkdir -p "$INSTALL_DIR"
  info "Downloading binary to $INSTALL_DIR/miri"
  curl -sL "$BINARY_URL" -o "$INSTALL_DIR/miri"
  chmod +x "$INSTALL_DIR/miri"

  # --- install config if missing ---
  if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
    CONFIG_URL=$(get_asset_url "$CONFIG_ASSET")
    mkdir -p "$CONFIG_DIR"
    info "No config found, downloading default config to $CONFIG_DIR/config.toml"
    curl -sL "$CONFIG_URL" -o "$CONFIG_DIR/config.toml"
  else
    info "Config already exists at $CONFIG_DIR/config.toml, skipping"
  fi

  if [[ "$SETUP_SERVICE" == "Yes (recommended)" ]]; then
    # --- install service ---
    SERVICE_URL=$(get_asset_url "$SERVICE_ASSET")
    mkdir -p "$WANTS_DIR"
    info "Downloading miri.service"
    curl -sL "$SERVICE_URL" -o "$SERVICE_DIR/miri.service"

    info "Symlinking into niri.service.wants"
    ln -sf "$SERVICE_DIR/miri.service" "$WANTS_DIR/miri.service"

    # --- reload ---
    info "Reloading systemd user daemon"
    systemctl --user daemon-reload

    success "Done. Miri will start with niri next login. To start now: systemctl --user start miri.service"
  else
    info "Skipped systemd service setup"
    success "Done. Binary installed to $INSTALL_DIR/miri"
  fi

elif [[ "$ACTION" == "Uninstall miri" ]]; then

  info "Stopping miri.service if running"
  systemctl --user stop miri.service 2>/dev/null || true

  info "Disabling miri.service"
  systemctl --user disable miri.service 2>/dev/null || true

  [[ -L "$WANTS_DIR/miri.service" ]] && rm -f "$WANTS_DIR/miri.service" && info "Removed symlink from niri.service.wants"
  [[ -f "$SERVICE_DIR/miri.service" ]] && rm -f "$SERVICE_DIR/miri.service" && info "Removed miri.service"
  [[ -f "$INSTALL_DIR/miri" ]] && rm -f "$INSTALL_DIR/miri" && info "Removed $INSTALL_DIR/miri"

  info "Reloading systemd user daemon"
  systemctl --user daemon-reload

  success "Done. Miri has been uninstalled"

fi