# Event-Driven Integration

This repository includes example integration files under [`scripts/`](../../scripts):

- `pico-inkpack-refresh.sh`
- `99-pico-inkpack-status.rules`
- `usb-state-sender-refresh.service`
- `usb-state-sender-refresh@.service`
- `50-usb-state-sender.networkd-dispatcher`
- `90-usb-state-sender.networkmanager-dispatcher`

The examples are designed for an event-driven flow:

1. `udev` creates a stable `/dev/pico-inkpack-status` symlink when the HID device appears.
2. `udev` starts a one-shot `systemd` service to push the current network state immediately after USB attach.
3. `networkd-dispatcher` or NetworkManager triggers the same refresh service after network changes.

That combination avoids polling and also handles the "USB attached after boot" case.

## Install the CLI

Build and install the sender binary:

```bash
cargo build --release
sudo install -m 0755 target/release/usb-state-sender /usr/local/bin/usb-state-sender
```

## Install the Refresh Script

Copy the shared refresh helper:

```bash
sudo install -D -m 0755 scripts/pico-inkpack-refresh.sh \
  /usr/local/libexec/usb-state-sender/pico-inkpack-refresh.sh
```

The script gathers the current IPv4 addresses, IPv6 addresses, hostname, and default route interface, then writes them to slots 1-4.

## Install `udev` and `systemd` Files

Copy the sample files into place:

```bash
sudo install -D -m 0644 scripts/99-pico-inkpack-status.rules \
  /etc/udev/rules.d/99-pico-inkpack-status.rules
sudo install -D -m 0644 scripts/usb-state-sender-refresh.service \
  /etc/systemd/system/usb-state-sender-refresh.service
sudo install -D -m 0644 scripts/usb-state-sender-refresh@.service \
  /etc/systemd/system/usb-state-sender-refresh@.service
```

Reload both managers:

```bash
sudo udevadm control --reload-rules
sudo systemctl daemon-reload
```

If the device is already connected, re-trigger `udev` once:

```bash
sudo udevadm trigger --subsystem-match=hidraw
```

## Install One Network Dispatcher Hook

Choose one dispatcher based on your network stack.

### `networkd-dispatcher`

Copy the example hook into a `routable.d` directory:

```bash
sudo install -D -m 0755 scripts/50-usb-state-sender.networkd-dispatcher \
  /etc/networkd-dispatcher/routable.d/50-usb-state-sender
```

### NetworkManager

Copy the example hook into the dispatcher directory:

```bash
sudo install -D -m 0755 scripts/90-usb-state-sender.networkmanager-dispatcher \
  /etc/NetworkManager/dispatcher.d/90-usb-state-sender
```

## Manual Refresh

You can test the full flow without waiting for an event:

```bash
sudo systemctl start usb-state-sender-refresh.service
```

Or run the refresh helper directly:

```bash
sudo /usr/local/libexec/usb-state-sender/pico-inkpack-refresh.sh
```

## Notes

- The `udev` rule uses a stable symlink instead of storing a changing `hidrawX` path.
- The `udev` attach event handles initial sync after late USB attachment.
- The network dispatcher handles later address changes without a polling timer.
- If you later need HID button input, event coalescing, or richer reconnection logic, that is the point where a daemon becomes more attractive.
