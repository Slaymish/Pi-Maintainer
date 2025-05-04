#!/bin/bash

cargo build --release

echo "Built!"

sudo install -m755 target/release/PiMainteno /usr/local/bin/pi-mainteno

sudo cp PiMainteno.toml /etc/pi-mainteno/PiMainteno.toml
sudo systemctl restart Pi-Maintainer.service
sudo systemctl status Pi-Maintainer.service
