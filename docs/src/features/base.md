<!-- vim-markdown-toc GFM -->

* [Nipart Base Interface support](#nipart-base-interface-support)
    * [`name` -- Interface name](#name----interface-name)
    * [`type` -- Interface type](#type----interface-type)
    * [`iface-index` - Interface index](#iface-index---interface-index)
    * [`state` - Interface state](#state---interface-state)
    * [`link-state` -- Link state](#link-state----link-state)
    * [`controller` - Controller interface](#controller---controller-interface)
    * [`mac-address` - MAC address](#mac-address---mac-address)
    * [`permanent-mac-address` - Permanent MAC address](#permanent-mac-address---permanent-mac-address)
    * [`mtu` - MTU](#mtu---mtu)
    * [`min-mtu` - Minimum MTU](#min-mtu---minimum-mtu)
    * [`max-mtu` - Maximum MTU](#max-mtu---maximum-mtu)
    * [`ipv4` and `ipv6` - IP configuration](#ipv4-and-ipv6---ip-configuration)

<!-- vim-markdown-toc -->

# Nipart Base Interface support

All interface types supported by Nipart support this YAML structure:

```yaml
---
version: 1
interfaces:
- name: eth1
  type: veth
  iface-index: 8
  state: up
  link-state: up
  controller: bond0
  mac-address: CE:67:51:EC:E2:6A
  permanent-mac-address: CE:67:51:EC:E2:6A
  mtu: 1500
  min-mtu: 68
  max-mtu: 65535
  ipv4:
    enabled: false
  ipv6:
    enabled: false
```


## `name` -- Interface name

The kernel name of interface

## `type` -- Interface type

The type of interface, e.g. `veth`, `bond`, `bridge`, etc.

## `iface-index` - Interface index

The kernel index of the interface. Query only property.

## `state` - Interface state

The management state of the interface:
 * `up`: the interface is administratively up
 * `down`: the interface is administratively down
 * `absent`: apply only property, request nipart to delete this interface or
   restore physical interface to kernel default state
 * `ignore`: Not managed by nipart
 * `up-ignore`: Interface is administratively up but not managed by nipart
 * `down-ignore`: Interface is administratively down but not managed by nipart

## `link-state` -- Link state

Query only property. Interface carrier state:
 * `up`: the interface has carrier
 * `down`: the interface does not have carrier
 * `dormant`: the interface is dormant, e.g. a wireless interface in power
   saving mode
 * `lower-layer-down`: the interface is down due to lower layer failure, e.g. a
   physical interface is down
 * `testing`: the interface is in testing mode

## `controller` - Controller interface

The kernel interface name of the controller interface.

When applying, setting this property to empty string means detach from its
current controller.
When applying, setting this property to a non-empty string means attach to the
controller interface.


## `mac-address` - MAC address

The current MAC address of the interface.

When applying, setting this property to empty string means restore the
permanent MAC address.
When applying, setting this property to a non-empty string means set the MAC
address.

## `permanent-mac-address` - Permanent MAC address

Query only property. The permanent MAC address of the interface


## `mtu` - MTU

MTU of the interface

## `min-mtu` - Minimum MTU

Query only property. The minimum MTU supported by the interface

## `max-mtu` - Maximum MTU

Query only property. The maximum MTU supported by the interface

## `ipv4` and `ipv6` - IP configuration

Please check [IP configuration](./ip.md) for details.
