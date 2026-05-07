# Nipart

<https://nispor.github.io/nipart/>

Nipart is a network management tool written in Rust providing descriptive
API for network management.

Nipart can work in common daemon-client mode, or one-shot daemon-free mode.

The `nipartd` daemon provides complex network management features, such as
DHCP, conditional network up/down.

The `npt` client by default, send request to daemon for querying and applying
network changes. With `npt apply <YAML> --no-daemon`, it bypass daemon and
apply network changes directly to Linux kernel or related daemon(e.g. OVS).

Example YAML connecting to a WIFI network(works both in daemon and no-daemon
mode):

```yaml
---
version: 1
routes:
  config:
  - destination: 0.0.0.0/0
    next-hop-interface: wlan0
    next-hop-address: 192.0.2.1
    metric: 100
interfaces:
- name: wlan0
  type: wifi-phy
  wifi:
    ssid: Test-WIFI
    password: 12345678
  ipv4:
    enabled: true
    dhcp: false
    address:
    - ip: 192.0.2.99
      prefix-length: 24
```


## Features

* [Base Interface Management](features/base.md)
* [IP Address](features/ip.md)
* [No Daemon Mode](features/no_daemon_mode.md)
* [WIFI](features/wifi.md)
* [Route](features/route.md)
* [Conditional Network Up/Down](features/trigger.md)
* [Wait Online](features/wait-online.md)
* [Vlan](features/vlan.md)
* [Bond](features/bond.md)
* [Linux Bridge](features/bridge.md)
* [OpenvSwitch Bridge](features/ovs.md)
* [Wireguard](features/wireguard.md)

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

## Usage

### Show current network state

```bash
# daemon mode
sudo npt show
# no-daemon mode
sudo npt show -n
```

### Show saved config of daemon

```bash
sudo npt show -s
```

### Show running status of certain interface

```bash
sudo npt show wlan0
```

### Scan WIFI networks

```bash
sudo npt wifi scan
```

### Connect to WIFI

```bash
# This command will ask you to input your wifi password
sudo npt wifi connect <SSID>
```
