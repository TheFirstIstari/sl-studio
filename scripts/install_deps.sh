#!/usr/bin/env bash
set -euo pipefail
DistID=""
if [ -f /etc/os-release ]; then
  . /etc/os-release
  DistID="$ID"
fi
if [[ "$DistID" == "ubuntu" || "$DistID" == "debian" ]]; then
  sudo apt-get update
  sudo apt-get install -y libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev
elif [[ "$DistID" == "fedora" ]]; then
  sudo dnf install -y webkit2gtk-devel libappindicator-gtk3-devel librsvg2-devel gtk3-devel
else
  echo "Unsupported Linux distro: $DistID" >&2
  exit 1
fi
