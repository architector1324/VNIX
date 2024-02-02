#!/bin/bash

qemu-system-x86_64 -enable-kvm -m 512M -full-screen -serial stdio -vga virtio -device virtio-rng-pci \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/ovmf/x64/OVMF.fd \
    -cdrom ./out/vnix.img
