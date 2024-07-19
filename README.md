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

Alternatively you can download a pre-built binary from the
[latest release](https://github.com/whot/huion-switcher/releases). Look for the
`huion-switcher.zip` that's attached to the release (and built by our
[CI workflow](https://github.com/whot/huion-switcher/blob/main/.github/workflows/main.yml)):

```
$ unzip huion-switcher.zip
$ chmod +x huion-switcher
$ sudo ./huion-switcher
```
Or see the `Installing` section below for installing ready for a udev invocation.

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
