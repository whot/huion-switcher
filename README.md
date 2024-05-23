# huion-switcher

**Use at your own risk**

A minimal PoC to switch Huion (Vendor ID `0x256C`) devices into raw tablet mode. If successful
this tool will print the following ouput on success:
```
HUION_MAGIC_BYTES=e7b480e280804ee1bfbfe18f98d883e8808004e0a1a3
HUION_FIRMWARE_ID=HUION_T21j_221221
```
Run via `IMPORT` in a udev rule to make the output udev properties on the
device (see the `Installing` section below).

If successful, the tablet should send its events via the HID Report ID `0x8`
(a 11 byte report on the Huion H641P).


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
