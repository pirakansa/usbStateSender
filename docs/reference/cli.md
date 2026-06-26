# CLI Reference

## Commands

```text
usb-state-sender [--device PATH] [--slot SLOT] <message...>
usb-state-sender --list
usb-state-sender --help
```

## Options

- `--device PATH`: write to an explicit hidraw device path.
- `--slot SLOT`: target one of `legacy`, `1`, `2`, `3`, or `4`. The default is `1`.
- `--list`: print auto-detected matching hidraw devices, one per line.
- `--help`: print usage information.

## Device Auto-Detection

When `--device` is omitted, the CLI scans `/sys/class/hidraw` and matches devices using:

- Vendor ID `2e8a`
- Product ID `00a0`
- Product string `Pico Inkpack Status`
- Interface string `Status HID`

If no devices match, the CLI exits with an error.
If multiple devices match, the CLI exits with an error and asks for `--device`.

## Report Format

The sender writes one 64-byte HID output report:

- byte `0`: command `0x01`
- byte `1`: target slot (`0x01`, `0x10`, `0x11`, `0x12`, `0x13`)
- byte `2`: payload length from `0` to `61`
- byte `3..63`: ASCII payload bytes padded with NUL

Non-ASCII characters are replaced with `?`.
Payloads longer than 61 bytes are truncated.

## Environment Overrides

These variables exist for tests and local development:

- `USB_STATE_SENDER_DEV_ROOT`
- `USB_STATE_SENDER_SYS_HIDRAW_ROOT`

When unset, the CLI uses `/dev` and `/sys/class/hidraw`.
