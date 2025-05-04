#!/usr/bin/env bash
set -euo pipefail
cargo build --release
echo "Built core"

# Build frontend static assets
echo "Building frontend..."
pushd frontend > /dev/null
npm run build
popd > /dev/null
echo "Built frontend"

sudo install -m755 target/release/PiMainteno /usr/local/bin/pi-mainteno
sudo mkdir -p /etc/pi-mainteno

sudo cp PiMainteno.toml /etc/pi-mainteno/PiMainteno.toml
echo "Deploying frontend assets..."
sudo mkdir -p /usr/share/pi-mainteno/static
sudo rm -rf /usr/share/pi-mainteno/static/*
sudo cp -r frontend/dist/* /usr/share/pi-mainteno/static/

sudo systemctl restart Pi-Maintainer.service
sudo systemctl status Pi-Maintainer.service
