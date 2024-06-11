# huion-switcher

A small utility to switch Huion (Vendor ID `0x256C`) devices into raw tablet
mode. If successful this tool will print the following ouput on success:
```
HUION_MAGIC_BYTES=e7b480e280804ee1bfbfe18f98d883e8808004e0a1a3
HUION_FIRMWARE_ID=HUION_T21j_221221
```
Run via `IMPORT` in a udev rule to make the output udev properties on the
device (see the `Installing` section below).

If successful, the tablet should send its events via the HID Report ID `0x8`
(a 11 byte report on the Huion H641P). To correctly interpret those events
you will need [this BPF program](https://gitlab.freedesktop.org/libevdev/udev-hid-bpf/-/merge_requests/85)
or something similar.

## Building and running

This tool needs access to the device and thus needs to run as root:
```
$ sudo cargo build
$ sudo cargo run
```

## Installing

```
$ cargo build
$ cp ./target/debug/huion-switcher /usr/lib/udev/
$ cp 80-huion-switcher.rules /etc/udev/rules.d
```
Then re-plug the device and udev will switch it to tablet mode on plug.

### HID_UNIQ and UNIQ

As part of the udev rule, the resulting `HUION_FIRMWARE_ID` property (if any)
is copied into the device's `HID_UNIQ` and `UNIQ` properties. This makes the
device recognizable by libwacom which relies on the firmware ID for matching.
