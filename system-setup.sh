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
apt-get install rustup
rustup default stable

# Project dependencies
apt-get install git
apt-get install x11-xserver-utils

# Now we'll need a reboot.
