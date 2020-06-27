#!/usr/bin/env bash

# Connect to the shared router.
# wpa_passphrase "NETGEAR48" "$2" > /etc/wpa_supplicant/wpa_supplicant.conf

# Enable ssh.
sudo touch /boot/ssh

# Set the hostname.
echo "$1" | sudo tee /etc/hostname
sudo sed -i "s/raspberrypi/$1/g" /etc/hosts

# Set the output to HDMI, 1080p
sudo sed -i "s/#hdmi_mode=1/hdmi_mode=82/g" /boot/config.txt
sudo sed -i "s/#hdmi_group=1/hdmi_group=2/g" /boot/config.txt
# Set audio output to HDMI.
amixer cset numid=3 2

# Hide the mouse cursor entirely.
sudo sed -i "s/#xserver-command=X/xserver-command=X -nocursor/g" /etc/lightdm/lightdm.conf

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
