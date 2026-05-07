<!-- vim-markdown-toc GFM -->

* [WIFI](#wifi)
    * [`ssid` -- SSID](#ssid----ssid)
    * [`state` -- Wifi state](#state----wifi-state)
    * [`bssid` -- BSSID](#bssid----bssid)
    * [`password` -- Password](#password----password)
    * [`base-iface` -- Base interface](#base-iface----base-interface)
    * [`auth-types` -- Authentication types](#auth-types----authentication-types)
    * [`generation` -- Wifi generation](#generation----wifi-generation)
    * [`frequency-mhz` -- Frequency](#frequency-mhz----frequency)
    * [`rx-bitrate-mb` -- Receive bitrate](#rx-bitrate-mb----receive-bitrate)
    * [`tx-bitrate-mb` -- Transmit bitrate](#tx-bitrate-mb----transmit-bitrate)
    * [`signal-dbm` -- Signal level](#signal-dbm----signal-level)
    * [`signal-percent` -- Signal percentage](#signal-percent----signal-percentage)

<!-- vim-markdown-toc -->
# WIFI

WIFI configuration in Nipart uses two interface types:

- **`wifi-phy`**: A kernel wifi physical interface (e.g. `wlan0`). Used for
  current state (query results) or when the physical interface name is known
  ahead of time.
- **`wifi-cfg`**: A psuedo/userspace-only interface that holds the desired wifi
  connection configuration. It has no kernel index and lives only in the Nipart
  desired state. You can optionally bind it to a specific `wifi-phy` via
  `base-iface`, or leave it unbound (meaning "any available wifi-phy").

Example YAML for WIFI configuration with static IP:

```yaml
version: 1
routes:
  config:
  - destination: 0.0.0.0/0
    next-hop-interface: wlan0
    next-hop-address: 172.17.2.1
    metric: 100
interfaces:
- name: wlan0
  type: wifi-phy
  state: up
  mtu: 1492
  ipv4:
    enabled: true
    dhcp: false
    address:
    - ip: 172.17.2.6
      prefix-length: 24
  wifi:
    ssid: SweatHome5G
    bssid: D0:21:F9:49:B3:52
    password: <_hidden_>
```

## `ssid` -- SSID

The SSID (Service Set Identifier) of the wifi network to connect to.

## `state` -- Wifi state

Query only property. The current connection state of the wifi link:

- `disconnected`: BSS disconnected
- `scanning`: Scanning for SSID
- `connecting`: SSID found, trying to associate and authenticate with a BSS/SSID
- `completed`: Data connection is fully configured and operational
- `unknown`: State could not be determined

## `bssid` -- BSSID

The BSSID (Basic Service Set Identifier) of the access point. When set, Nipart
will only connect to the specified AP. If omitted, any AP broadcasting the SSID
may be used.

## `password` -- Password

The password or pre-shared key for authentication. This field is replaced
with `<_hidden_>` when querying current state.

## `base-iface` -- Base interface

The kernel name of the wifi physical interface to bind this configuration to.
If set, the wifi connection is restricted to that specific interface.

When using `wifi-phy` type, this field defaults to the interface name itself.
When using `wifi-cfg` type with `base-iface: <name>`, the config binds to that
physical interface. When undefined (unbound), the config applies to any eligible
`wifi-phy` interface.

## `auth-types` -- Authentication types

Query only property. It contains the current authentication type. When showing
wifi scan results, it contains the authentication types supported by the AP.
Ignored when applying.

Supported authentication types:

- `OPEN` -- No authentication (open network)
- `WEP` -- WEP (deprecated)
- `WPA1` -- WPA 1 (deprecated)
- `WPS` -- WPS (deprecated)
- `WPA2-PSK` -- WPA 2 Pre-shared Key
- `EAP` -- WPA 2/3 EAP / Enterprise (including OSEN)
- `WPA3-PSK` -- WPA 3 Pre-shared Key using SAE
- `WPA3-OPEN` -- WPA 3 open network using OWE
- `FILS` -- IEEE 802.11ai Fast Initial Link Setup
- `DPP` -- Device Provisioning Protocol (Easy Connect)

## `generation` -- Wifi generation

Query only property. The wifi generation, e.g. `6` for WiFi 6.

## `frequency-mhz` -- Frequency

Query only property. The wifi frequency in MHz.

## `rx-bitrate-mb` -- Receive bitrate

Query only property. The receive bitrate in 1 Mb/s.

## `tx-bitrate-mb` -- Transmit bitrate

Query only property. The transmit bitrate in 1 Mb/s.

## `signal-dbm` -- Signal level

Query only property. The signal strength in dBm.

## `signal-percent` -- Signal percentage

Query only property. The signal strength as a percentage (0-100).
