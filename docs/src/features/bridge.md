<!-- vim-markdown-toc GFM -->

* [Linux Bridge](#linux-bridge)
    * [`options` -- Bridge options](#options----bridge-options)
        * [`group-addr` -- Multicast group address](#group-addr----multicast-group-address)
        * [`group-fwd-mask` -- Group forward mask](#group-fwd-mask----group-forward-mask)
        * [`hash-max` -- Hash table maximum](#hash-max----hash-table-maximum)
        * [`mac-ageing-time` -- MAC ageing time](#mac-ageing-time----mac-ageing-time)
        * [`multicast-last-member-count` -- Last member query count](#multicast-last-member-count----last-member-query-count)
        * [`multicast-last-member-interval` -- Last member query interval](#multicast-last-member-interval----last-member-query-interval)
        * [`multicast-membership-interval` -- Membership interval](#multicast-membership-interval----membership-interval)
        * [`multicast-querier` -- Multicast querier](#multicast-querier----multicast-querier)
        * [`multicast-querier-interval` -- Querier interval](#multicast-querier-interval----querier-interval)
        * [`multicast-query-interval` -- Query interval](#multicast-query-interval----query-interval)
        * [`multicast-query-response-interval` -- Query response interval](#multicast-query-response-interval----query-response-interval)
        * [`multicast-query-use-ifaddr` -- Query use interface address](#multicast-query-use-ifaddr----query-use-interface-address)
        * [`multicast-router` -- Multicast router type](#multicast-router----multicast-router-type)
        * [`multicast-snooping` -- Multicast snooping](#multicast-snooping----multicast-snooping)
        * [`multicast-startup-query-count` -- Startup query count](#multicast-startup-query-count----startup-query-count)
        * [`multicast-startup-query-interval` -- Startup query interval](#multicast-startup-query-interval----startup-query-interval)
        * [`stp` -- Spanning Tree Protocol](#stp----spanning-tree-protocol)
            * [`enabled` -- STP enabled](#enabled----stp-enabled)
            * [`forward-delay` -- Forward delay](#forward-delay----forward-delay)
            * [`hello-time` -- Hello time](#hello-time----hello-time)
            * [`max-age` -- Maximum age](#max-age----maximum-age)
            * [`priority` -- Bridge priority](#priority----bridge-priority)
        * [`vlan-protocol` -- VLAN protocol](#vlan-protocol----vlan-protocol)
        * [`vlan-default-pvid` -- Default PVID](#vlan-default-pvid----default-pvid)
    * [`ports` -- Bridge ports](#ports----bridge-ports)
        * [`name` -- Port name](#name----port-name)
        * [`stp-hairpin-mode` -- Hairpin mode](#stp-hairpin-mode----hairpin-mode)
        * [`stp-path-cost` -- STP path cost](#stp-path-cost----stp-path-cost)
        * [`stp-priority` -- STP priority](#stp-priority----stp-priority)
        * [`vlan` -- Port VLAN filtering](#vlan----port-vlan-filtering)
    * [`vlan` -- Bridge VLAN filtering](#vlan----bridge-vlan-filtering)
        * [`mode` -- VLAN mode](#mode----vlan-mode)
        * [`tag` -- Native VLAN tag](#tag----native-vlan-tag)
        * [`enable-native` -- Enable native VLAN](#enable-native----enable-native-vlan)
        * [`trunk-tags` -- Trunk tags](#trunk-tags----trunk-tags)

<!-- vim-markdown-toc -->
# Linux Bridge

Example YAML of Linux bridge interface configuration:

```yaml
version: 1
interfaces:
- name: br0
  type: linux-bridge
  state: up
  bridge:
    options:
      group-addr: 01:80:C2:00:00:00
      group-fwd-mask: 0
      hash-max: 4096
      mac-ageing-time: 300
      multicast-last-member-count: 2
      multicast-last-member-interval: 100
      multicast-membership-interval: 26000
      multicast-querier: false
      multicast-querier-interval: 25500
      multicast-query-interval: 12500
      multicast-query-response-interval: 1000
      multicast-query-use-ifaddr: false
      multicast-router: auto
      multicast-snooping: true
      multicast-startup-query-count: 2
      multicast-startup-query-interval: 3125
      stp:
        enabled: true
        forward-delay: 15
        hello-time: 2
        max-age: 20
        priority: 32768
      vlan-protocol: 802.1q
      vlan-default-pvid: 1
    ports:
    - name: eth1
      stp-hairpin-mode: false
      stp-path-cost: 100
      stp-priority: 32
    - name: eth2
      stp-hairpin-mode: false
      stp-path-cost: 100
      stp-priority: 32
```

## `options` -- Bridge options

Linux bridge kernel options. When applying, existing options are merged into
desired.

### `group-addr` -- Multicast group address

The multicast MAC address used by the bridge. Default is `01:80:C2:00:00:00`.

### `group-fwd-mask` -- Group forward mask

Also configurable as `group-forward-mask` (deprecated alias). Defines the mask
for forwarding link-local frames. Setting a bit enables forwarding of frames
with the corresponding destination MAC address.

### `hash-max` -- Hash table maximum

The maximum size of the multicast hash table.

### `mac-ageing-time` -- MAC ageing time

The MAC address ageing time in seconds. Controls how long a learned MAC
address is kept in the forwarding database without being refreshed.

### `multicast-last-member-count` -- Last member query count

The number of queries sent after receiving a leave message.

### `multicast-last-member-interval` -- Last member query interval

The interval in milliseconds between last member query transmissions.

### `multicast-membership-interval` -- Membership interval

The interval in milliseconds after which a multicast membership expires.

### `multicast-querier` -- Multicast querier

When set to `true`, the bridge can act as a multicast querier.

### `multicast-querier-interval` -- Querier interval

The interval in milliseconds between querier transmissions.

### `multicast-query-interval` -- Query interval

The interval in milliseconds between general multicast queries.

### `multicast-query-response-interval` -- Query response interval

The maximum response time in milliseconds for multicast queries.

### `multicast-query-use-ifaddr` -- Query use interface address

When set to `true`, the bridge uses its own IP address as the source of
multicast queries.

### `multicast-router` -- Multicast router type

The multicast router type:
 * `auto` (1): The bridge automatically detects multicast routers.
 * `disabled` (0): Multicast router functionality is disabled.
 * `enabled` (2): The bridge acts as a multicast router.

### `multicast-snooping` -- Multicast snooping

When set to `true`, the bridge performs IGMP/MLD snooping to reduce multicast
traffic.

### `multicast-startup-query-count` -- Startup query count

The number of queries sent when the bridge starts.

### `multicast-startup-query-interval` -- Startup query interval

The interval in milliseconds between startup queries.

### `stp` -- Spanning Tree Protocol

STP options for the bridge.

#### `enabled` -- STP enabled

Enables or disables Spanning Tree Protocol on the bridge. When disabled, the
remaining STP options are discarded during apply.

#### `forward-delay` -- Forward delay

The forwarding delay in seconds. Valid range is 2 to 30.

#### `hello-time` -- Hello time

The interval in seconds between STP hello BPDU transmissions. Valid range is 1
to 10.

#### `max-age` -- Maximum age

The maximum age of STP information in seconds. Valid range is 6 to 40.

#### `priority` -- Bridge priority

The STP bridge priority. Lower priority increases the chance of becoming the
root bridge.

### `vlan-protocol` -- VLAN protocol

The VLAN encapsulation protocol used by the bridge:
 * `802.1q`: Standard IEEE 802.1Q VLAN tagging (default).
 * `802.1ad`: Provider Bridging (Q-in-Q) IEEE 802.1ad.

### `vlan-default-pvid` -- Default PVID

The default Port VLAN ID (PVID) assigned to ports. Default is `1`. Cannot be
changed to a value other than `1` unless VLAN filtering is enabled.

## `ports` -- Bridge ports

List of bridge port configurations. When applying, the desired port list will
override the current port list.

### `name` -- Port name

The kernel interface name of the bridge port. Mandatory.

### `stp-hairpin-mode` -- Hairpin mode

When set to `true`, traffic may be sent back out of the port on which it was
received.

### `stp-path-cost` -- STP path cost

The STP path cost of the port. Used in root port and designated port selection.

### `stp-priority` -- STP priority

The STP port priority. An unsigned 8-bit value (0 to 255). Lower priority
increases the chance of becoming the designated port.

### `vlan` -- Port VLAN filtering

VLAN filtering configuration specific to this port. If not defined, the
current VLAN filtering is preserved for the port.

## `vlan` -- Bridge VLAN filtering

The VLAN filtering configuration for the bridge itself. Setting to
`vlan: {}` will remove all VLANs.

### `mode` -- VLAN mode

The bridge VLAN filtering mode:
 * `access`: Single untagged VLAN (access port).
 * `trunk`: Tagged VLANs (trunk port).

Defaults to `access` if not defined.

### `tag` -- Native VLAN tag

The VLAN tag for the native VLAN. In `access` mode, this is the access VLAN.
In `trunk` mode, requires `enable-native` to be `true`.

### `enable-native` -- Enable native VLAN

When set to `true`, the `tag` VLAN is treated as the native untagged VLAN on a
trunk port. Cannot be set in `access` mode.

### `trunk-tags` -- Trunk tags

List of allowed VLANs on a trunk port. Each entry is either a single VLAN ID
or a range:

```yaml
trunk-tags:
- id: 100
- id-range:
    min: 200
    max: 300
```

Overlapping trunk tags are not allowed.
