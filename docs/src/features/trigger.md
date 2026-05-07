<!-- vim-markdown-toc GFM -->

* [Conditional Network Up/Down](#conditional-network-updown)
    * [Example: Carrier-based up/down](#example-carrier-based-updown)
    * [Example: Start VPN when not in home WIFI](#example-start-vpn-when-not-in-home-wifi)

<!-- vim-markdown-toc -->

# Conditional Network Up/Down

For conditionally bringing an interface up/down, the `trigger` section is
used for daemon mode only.

The `trigger` takes these values:

* `never`: Never do auto action to bring interface up/down.
* `always`: Always bring interface up/down regardless of carrier.
* `carrier`: Bring interface up if carrier is up, otherwise bring
  interface down.
* `wifi: <SSID>`: Bring interface up if specified SSID is connected,
  otherwise disconnect.
* `wifi-not: <SSID>`: Bring interface down if specified SSID is connected,
  otherwise connect.

## Example: Carrier-based up/down

```yaml
interfaces:
- name: enp7s0
  type: ethernet
  state: up
  trigger: carrier
  ipv4:
    enabled: true
    dhcp: false
    address:
    - ip: 192.0.2.251
      prefix-length: 32
  ipv6:
    enabled: false
```

## Example: Start VPN when not in home WIFI

```yaml
routes:
  config:
  - destination: 203.0.113.0/24
    next-hop-interface: wg0
    next-hop-address: 198.51.100.1
    metric: 100
    table-id: 25
interfaces:
- name: wg0
  type: wireguard
  state: up
  ipv4:
    enabled: true
    dhcp: false
    address:
    - ip: 198.51.100.9
      prefix-length: 24
  trigger:
    wifi-not: HomeWifi
  wireguard:
    public-key: JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA=
    private-key: 6LTHiAM4vgKEgi5vm30f/EBIEWFDmySkTc9EWCcIqEs=
    listen-port: 51820
    peers:
    - endpoint: 192.0.2.0:51820
      public-key: 8bdQrVLqiw3ZoHCucNh1YfH0iCWuyStniRr8t7H24Fk=
      persistent-keepalive: 0
      allowed-ips:
      - ip: 0.0.0.0
        prefix-length: 0
```
