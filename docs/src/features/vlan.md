<!-- vim-markdown-toc GFM -->

* [Vlan](#vlan)
    * [`base-iface` -- Base interface](#base-iface----base-interface)
    * [`id` -- VLAN ID](#id----vlan-id)
    * [`protocol` -- VLAN protocol](#protocol----vlan-protocol)
    * [`registration-protocol` -- VLAN registration protocol](#registration-protocol----vlan-registration-protocol)
    * [`reorder-headers` -- Reorder output packet headers](#reorder-headers----reorder-output-packet-headers)
    * [`loose-binding` -- Loose binding](#loose-binding----loose-binding)
    * [`bridge-binding` -- Bridge binding](#bridge-binding----bridge-binding)
    * [`ingress-qos-map` -- Ingress QoS mapping](#ingress-qos-map----ingress-qos-mapping)
    * [`egress-qos-map` -- Egress QoS mapping](#egress-qos-map----egress-qos-mapping)

<!-- vim-markdown-toc -->

# Vlan

Example YAML of VLAN interface configuration:

```yaml
version: 1
interfaces:
- name: eth1.101
  type: vlan
  state: up
  vlan:
    base-iface: eth1
    id: 101
    protocol: 802.1q
    registration-protocol: none
    reorder-headers: true
    loose-binding: false
    bridge-binding: false
    ingress-qos-map:
    - from: 3
      to: 1
    egress-qos-map:
    - from: 1
      to: 3
```

## `base-iface` -- Base interface

The physical or parent interface name on which the VLAN is created, e.g.
`eth1`.

Mandatory when creating a new VLAN interface. When applying changes to an
existing VLAN, leaving this unset preserves the current base interface.

## `id` -- VLAN ID

The VLAN identifier. Valid range is 0 to 4094.

Mandatory when creating a new VLAN interface. When applying changes to an
existing VLAN, leaving this unset preserves the current ID.

## `protocol` -- VLAN protocol

The VLAN encapsulation protocol:
 * `802.1q`: Standard IEEE 802.1Q VLAN tagging (default).
 * `802.1ad`: Provider Bridging (Q-in-Q) IEEE 802.1ad.

Defaults to `802.1q` if not defined.

## `registration-protocol` -- VLAN registration protocol

The registration protocol used for VLAN pruning:
 * `gvrp`: GARP VLAN Registration Protocol.
 * `mvrp`: Multiple VLAN Registration Protocol.
 * `none`: No registration protocol (default).

## `reorder-headers` -- Reorder output packet headers

When set to `true`, the VLAN device will reorder the output packet headers to
move the VLAN tag before any hardware-specific headers. Defaults to `true`.

## `loose-binding` -- Loose binding

When set to `true`, the VLAN device operates in loose binding mode, where the
VLAN device state is not strictly tied to the master device's operating state.

## `bridge-binding` -- Bridge binding

When set to `true`, the VLAN device link state tracks the state of bridge ports
that are members of the VLAN.

## `ingress-qos-map` -- Ingress QoS mapping

Maps VLAN header PCP (Priority Code Point) values to Linux internal packet
priority for incoming packets. Each entry maps `from` (VLAN PCP value) to `to`
(Linux priority).

The maximum priority value is 7 according to 802.1Q-2018 PCP field definition.

## `egress-qos-map` -- Egress QoS mapping

Maps Linux internal packet priority to VLAN header PCP values for outgoing
packets. Each entry maps `from` (Linux priority) to `to` (VLAN PCP value).

The maximum priority value is 7 according to 802.1Q-2018 PCP field definition.
