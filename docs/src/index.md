# Nipart Quick User Guide

## Installation

### Build from source
```bash
cargo build --release
sudo systemctl stop nipart || true
sudo cp -fv target/release/nipartd /usr/bin/
sudo cp -fv target/release/npt /usr/bin/
sudo cp -fv packaging/nipart.service /etc/systemd/system/
sudo cp -fv packaging/nipart-wait-online.service /etc/systemd/system/
sudo systemctl enable nipart.service
sudo systemctl enable nipart-wait-online.service
sudo systemctl start nipart.service
```

### Install from Archlinux AUR

TODO: Upload to AUR

### Install from Fedora COPR

TODO: Upload to COPR

## Connect WIFI

1. Scan existing WIFI networks

```bash
sudo npt wifi scan
```

2. Connect to a WIFI network

```bash
# This command will ask you to input your password
sudo npt wifi connect <SSID>
```

3. Check saved WIFI config

```bash
sudo npt show wlan0 -s
```

4. Check running status of WIFI connection

```bash
sudo npt show wlan0
```

## Connect wireguard VPN

**Note**: Nipart does not support auto-route yet. You need to add route
manually.

```bash
echo '
interfaces:
- name: wg0
  type: wireguard
  state: up
  wireguard:
    public-key: "JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA="
    private-key: "6LTHiAM4vgKEgi5vm30f/EBIEWFDmySkTc9EWCcIqEs="
    listen-ports: 51820
    peers:
      - endpoint: 192.0.2.0:51820
        public-key: 8bdQrVLqiw3ZoHCucNh1YfH0iCWuyStniRr8t7H24Fk=
        preshared-key: TqIkTsTSxWJ1vSnhUW2oXFAtB5l9hRFWdgn2BrKX3ik=
        persistent-keepalive: 0
        allowed-ips:
        - ip: 0.0.0.0
          prefix-length: 0
        - ip: '::'
          prefix-length: 0
        protocol-version: 1
' | sudo npt apply -
```
