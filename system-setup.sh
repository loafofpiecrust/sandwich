#!/usr/bin/env bash

# Connect to the shared router.
# wpa_passphrase "NETGEAR48" "$2" > /etc/wpa_supplicant/wpa_supplicant.conf

# Enable ssh.
sudo touch /boot/ssh

# Set the hostname.
echo "$1" | sudo tee /etc/hostname
sudo sed -i "s/raspberrypi/$1/g" /etc/hosts

echo "hdmi_mode=82" | sudo tee -a /boot/config.txt

# Advertise this machine to the local network.
sudo apt-get install avahi-daemon

# Install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Project dependencies
sudo apt-get install git
sudo apt-get install x11-xserver-utils
sudo apt-get install libasound2-dev
# sudo apt-get install vim

# Now we'll need a reboot.
