# huion-switcher

A small utility to switch Huion (Vendor ID `0x256C`) devices into raw tablet
mode. Typically these devices will report three hidraw devices (see the
[`hid-recorder`](https://github.com/hidutils/hid-recorder) output) with
different report descriptors:
- the first one will consist of only a `Vendor Defined Page` (i.e. "top secret
  sauce") that the kernel will not (cannot) interpret. This device does not
  send events.
- the second one will be `Usage Page (Digitizer)`, i.e. the pen. Pen events are
  sent via this device.
- the third one will be `Usage Page (Generic Desktop)` and emulates a keyboard.
  Pad button presses will send keyboard events on this device.

`huion-switcher` reads a special string descriptor index from the US English
(`0x409`) language ID in the USB report descriptor. Doing so causes the
device to stop sending events via the pen/keyboard device and instead send
events via the vendor hidraw device.
This gives us access to actual pad buttons and more rather than the (ambiguous)
emulated key presses. However, for the kernel to correctly interpret those
events you will need [this BPF program](https://gitlab.freedesktop.org/libevdev/udev-hid-bpf/-/merge_requests/85)
or something similar for your device.

**If run without a corresponding BPF program, the kernel will discard
all events and the device must be unplugged and re-plugged to go back to
the emulation mode.**

If successful this tool will print the following ouput on success:
```
HUION_MAGIC_BYTES=e7b480e280804ee1bfbfe18f98d883e8808004e0a1a3
HUION_FIRMWARE_ID=HUION_T21j_221221
```
Run via `IMPORT` in a udev rule to make the output udev properties on the
device (see the `Installing` section below).

The `HUION_FIRMWARE_ID` is used by e.g. libwacom to detect the device (Huion
re-uses USB product IDs so the firmware is often the only distinction we have).

The `HUION_MAGIC_BYTES` contain information about the device's capabilities that
can be used by the corresponding BPF to set up the device.

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
