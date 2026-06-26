#!/bin/sh
set -eu

device_path="${1:-/dev/pico-inkpack-status}"
sender_bin="${USB_STATE_SENDER_BIN:-/usr/local/bin/usb-state-sender}"

collect_ipv4() {
    ip -o -4 addr show up scope global 2>/dev/null | awk '
        {
            split($4, address, "/");
            if (output != "") {
                output = output ", ";
            }
            output = output address[1];
        }
        END {
            print output;
        }
    '
}

collect_ipv6() {
    ip -o -6 addr show up scope global 2>/dev/null | awk '
        {
            split($4, address, "/");
            if (output != "") {
                output = output ", ";
            }
            output = output address[1];
        }
        END {
            print output;
        }
    '
}

collect_default_interface() {
    ip route show default 2>/dev/null | awk 'NR == 1 { print $5 }'
}

if [ ! -e "$device_path" ]; then
    exit 0
fi

if [ ! -x "$sender_bin" ]; then
    echo "usb-state-sender binary not found at $sender_bin" >&2
    exit 1
fi

hostname_value="$(hostname 2>/dev/null || printf '%s\n' unknown)"
ipv4_value="$(collect_ipv4)"
ipv6_value="$(collect_ipv6)"
default_interface="$(collect_default_interface)"

[ -n "$ipv4_value" ] || ipv4_value="unknown"
[ -n "$ipv6_value" ] || ipv6_value="none"
[ -n "$default_interface" ] || default_interface="unknown"

"$sender_bin" --device "$device_path" --slot 1 "IPv4: $ipv4_value"
"$sender_bin" --device "$device_path" --slot 2 "IPv6: $ipv6_value"
"$sender_bin" --device "$device_path" --slot 3 "Host: $hostname_value"
"$sender_bin" --device "$device_path" --slot 4 "Link: $default_interface"
