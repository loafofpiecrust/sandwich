#!/usr/bin/env bash

# Connect to the shared router.
# wpa_passphrase "NETGEAR48" "$2" > /etc/wpa_supplicant/wpa_supplicant.conf

# Enable ssh.
touch /boot/ssh

# Set the hostname.
echo "$1" > /etc/hostname
sed -i "s/raspberrypi/$1/g" /etc/hosts

# Advertise this machine to the local network.
apt-get install avahi-daemon

# Install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Project dependencies
apt-get install git
apt-get install x11-xserver-utils
apt-get install nvim

# Now we'll need a reboot.
