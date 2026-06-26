# usbStateSender

`usbStateSender` is the host-side Rust CLI for sending status text to the
`pico_inkpack_IDE` HID interface.

## Setup

- Rust stable
- `vorbere` for task shortcuts

Build and test the project once after cloning:

```bash
vorbere run check
vorbere run test
vorbere run build
```

## Quick Start

Send a one-off status line to the first matching device:

```bash
cargo run -- "IP: 192.0.2.10"
```

## Common Commands

- Show CLI help: `vorbere run run`
- Test: `vorbere run test`
- Check formatting and lints: `vorbere run check`
- CI-equivalent checks: `vorbere run ci`
- Create Linux amd64 and arm64 release archives: `vorbere run release`

## Documentation

- [Event-driven integration guide](docs/user-guides/event-driven-integration.md)
- [CLI reference](docs/reference/cli.md)
- [Documentation index](docs/README.md)

## Repository Layout

- `src/`: CLI implementation
- `tests/`: integration coverage
- `scripts/`: example `udev`, `systemd`, and dispatcher integration files
- `docs/`: detailed user guides and references
