<!-- vim-markdown-toc GFM -->

* [Bond](#bond)
    * [`mode` -- Bond mode](#mode----bond-mode)
    * [`options` -- Bond options](#options----bond-options)
        * [`miimon` -- MII link monitoring interval](#miimon----mii-link-monitoring-interval)
        * [`updelay` -- Up delay](#updelay----up-delay)
        * [`downdelay` -- Down delay](#downdelay----down-delay)
        * [`use_carrier` -- Use carrier](#use_carrier----use-carrier)
        * [`arp_interval` -- ARP monitoring interval](#arp_interval----arp-monitoring-interval)
        * [`arp_ip_target` -- ARP target IPs](#arp_ip_target----arp-target-ips)
        * [`arp_all_targets` -- ARP all targets](#arp_all_targets----arp-all-targets)
        * [`arp_validate` -- ARP validation](#arp_validate----arp-validation)
        * [`arp_missed_max` -- ARP missed max](#arp_missed_max----arp-missed-max)
        * [`fail_over_mac` -- Fail over MAC](#fail_over_mac----fail-over-mac)
        * [`primary` -- Primary port](#primary----primary-port)
        * [`primary_reselect` -- Primary reselect policy](#primary_reselect----primary-reselect-policy)
        * [`lacp_rate` -- LACP rate](#lacp_rate----lacp-rate)
        * [`lacp_active` -- LACP active](#lacp_active----lacp-active)
        * [`xmit_hash_policy` -- Transmit hash policy](#xmit_hash_policy----transmit-hash-policy)
        * [`ad_select` -- 802.3ad aggregation selection](#ad_select----8023ad-aggregation-selection)
        * [`ad_actor_sys_prio` -- Actor system priority](#ad_actor_sys_prio----actor-system-priority)
        * [`ad_actor_system` -- Actor system MAC](#ad_actor_system----actor-system-mac)
        * [`ad_user_port_key` -- User port key](#ad_user_port_key----user-port-key)
        * [`all_slaves_active` -- All slaves active](#all_slaves_active----all-slaves-active)
        * [`min_links` -- Minimum links](#min_links----minimum-links)
        * [`lp_interval` -- LACP PDU interval](#lp_interval----lacp-pdu-interval)
        * [`packets_per_slave` -- Packets per slave](#packets_per_slave----packets-per-slave)
        * [`resend_igmp` -- Resend IGMP](#resend_igmp----resend-igmp)
        * [`num_grat_arp` -- Gratuitous ARP count](#num_grat_arp----gratuitous-arp-count)
        * [`num_unsol_na` -- Unsolicited NA count](#num_unsol_na----unsolicited-na-count)
        * [`peer_notif_delay` -- Peer notification delay](#peer_notif_delay----peer-notification-delay)
        * [`tlb_dynamic_lb` -- TLB dynamic load balancing](#tlb_dynamic_lb----tlb-dynamic-load-balancing)
        * [`ns_ip6_target` -- IPv6 neighbor solicitation targets](#ns_ip6_target----ipv6-neighbor-solicitation-targets)
    * [`ports` -- Bond ports](#ports----bond-ports)
        * [`name` -- Port name](#name----port-name)
        * [`priority` -- Port priority](#priority----port-priority)
        * [`queue-id` -- Port queue ID](#queue-id----port-queue-id)

<!-- vim-markdown-toc -->
# Bond

Example YAML of bond interface configuration:

```yaml
version: 1
interfaces:
- name: bond0
  type: bond
  state: up
  bond:
    mode: 802.3ad
    options:
      miimon: 100
      updelay: 0
      downdelay: 0
      use_carrier: true
      lacp_rate: slow
      lacp_active: true
      xmit_hash_policy: layer3+4
      ad_select: stable
      ad_actor_sys_prio: 65535
      ad_user_port_key: 0
      min_links: 0
      lp_interval: 1
      packets_per_slave: 1
      resend_igmp: 1
      all_slaves_active: dropped
      arp_interval: 0
      arp_ip_target: 192.0.2.1,192.0.2.2
      arp_all_targets: any
      arp_validate: none
      arp_missed_max: 3
      fail_over_mac: none
      primary: eth1
      primary_reselect: always
      num_grat_arp: 1
      num_unsol_na: 1
      peer_notif_delay: 0
      tlb_dynamic_lb: true
      ns_ip6_target:
      - "2001:db8::1"
    ports:
    - name: eth1
      priority: 0
      queue-id: 0
    - name: eth2
      priority: 0
      queue-id: 0
```

## `mode` -- Bond mode

The bonding mode. Mandatory when creating a new bond interface. Supports
numeric aliases for deserialization:

 * `balance-rr` (0): Round-robin -- transmit packets in sequential order.
 * `active-backup` (1): Active-backup -- one port is active, standby takes
   over on failure.
 * `balance-xor` (2): XOR -- transmit based on XOR of MAC addresses.
 * `broadcast` (3): Broadcast -- transmits all packets on all ports.
 * `802.3ad` (4, alias `lacp`): IEEE 802.3ad dynamic link aggregation.
 * `balance-tlb` (5): Adaptive transmit load balancing.
 * `balance-alb` (6): Adaptive load balancing (TLB + receive load balancing).

## `options` -- Bond options

Kernel bond options. Please refer to kernel documentation for details.

### `miimon` -- MII link monitoring interval

Interval in milliseconds for MII link monitoring. Default is 0 (disabled).
Cannot be used together with `arp_interval`.

### `updelay` -- Up delay

Delay in milliseconds before enabling a port after link up detection.
Default is 0.

### `downdelay` -- Down delay

Delay in milliseconds before disabling a port after link loss detection.
Default is 0.

### `use_carrier` -- Use carrier

Whether to use MII/ETHTOOL ioctl or netif_carrier_ok() for link detection.
Default is `true`.

### `arp_interval` -- ARP monitoring interval

Interval in milliseconds for ARP monitoring. Default is 0 (disabled).
Cannot be used together with `miimon`. Invalid for `802.3ad`, `balance-tlb`,
and `balance-alb` modes.

### `arp_ip_target` -- ARP target IPs

Comma-separated IPv4 addresses used as ARP monitoring targets. Only valid
when `arp_interval` is greater than 0.

### `arp_all_targets` -- ARP all targets

Specifies how many `arp_ip_target` must be reachable for a port to be
considered up:
 * `any` (0): Any single target is reachable.
 * `all` (1): All targets must be reachable.

Only affects `active-backup` mode with `arp_validate` enabled.

### `arp_validate` -- ARP validation

Specifies ARP probe and reply validation for link monitoring:
 * `none` (0): No validation.
 * `active` (1): Validate only on the active port.
 * `backup` (2): Validate only on backup ports.
 * `all` (3): Validate on all ports.
 * `filter` (4): Filter non-ARP traffic on all ports.
 * `filter_active` (5): Filter on all ports, validate on active port.
 * `filter_backup` (6): Filter on all ports, validate on backup ports.

### `arp_missed_max` -- ARP missed max

Maximum number of ARP monitoring misses before considering a link down.

### `fail_over_mac` -- Fail over MAC

Specifies MAC address handling in `active-backup` mode:
 * `none` (0): All ports share the same MAC address.
 * `active` (1): Bond MAC follows the active port's MAC.
 * `follow` (2): Ports are programmed with bond MAC only on failover.

When `fail_over_mac` is set to `active` with `active-backup` mode, the desired
MAC address is ignored as it is determined by the active port.

### `primary` -- Primary port

The port name to use as the primary port (preferred active port) in
`active-backup`, `balance-tlb`, and `balance-alb` modes.

### `primary_reselect` -- Primary reselect policy

Specifies when the primary port becomes active again:
 * `always` (0): Primary becomes active whenever it comes back up.
 * `better` (1): Only if primary has better speed/duplex than current active.
 * `failure` (2): Only if current active port fails.

### `lacp_rate` -- LACP rate

LACP PDU rate in `802.3ad` mode:
 * `slow` (0): Transmit LACPDUs every 30 seconds.
 * `fast` (1): Transmit LACPDUs every 1 second.

Only valid in `802.3ad` mode.

### `lacp_active` -- LACP active

When set to `true`, LACPDUs are transmitted regardless of the partner state.
When `false`, LACPDUs are only transmitted in response to received LACPDUs.
Only valid in `802.3ad` mode.

### `xmit_hash_policy` -- Transmit hash policy

Transmit hash policy for `balance-xor`, `802.3ad`, and `balance-tlb` modes:
 * `layer2` (0): Hash based on MAC addresses.
 * `layer3+4` (1): Hash based on IP and port.
 * `layer2+3` (2): Hash based on MAC and IP.
 * `encap2+3` (3): Encapsulated layer 2+3 for tunnel interfaces.
 * `encap3+4` (4): Encapsulated layer 3+4 for tunnel interfaces.
 * `vlan+srcmac` (5): Hash based on VLAN tag and source MAC.

### `ad_select` -- 802.3ad aggregation selection

Aggregation selection logic for `802.3ad` mode:
 * `stable` (0): Use the stable aggregator.
 * `bandwidth` (1): Select aggregator with highest bandwidth.
 * `count` (2): Select aggregator with most ports.

### `ad_actor_sys_prio` -- Actor system priority

The 802.3ad actor system priority. Used in LACP negotiation.

### `ad_actor_system` -- Actor system MAC

The 802.3ad actor system MAC address. Cannot be a multicast address (prefix
`01:00:5E`).

### `ad_user_port_key` -- User port key

The 802.3ad user-defined port key.

### `all_slaves_active` -- All slaves active

Also deserializable as `all_ports_active`. Specifies handling of duplicate
frames on inactive ports:
 * `dropped` (0): Drop duplicate frames.
 * `delivered` (1): Deliver duplicate frames.

### `min_links` -- Minimum links

Minimum number of ports that must be active before the bond is considered
up.

### `lp_interval` -- LACP PDU interval

The number of seconds between LACP PDU transmissions.

### `packets_per_slave` -- Packets per slave

Also deserializable as `packets_per_port`. Number of packets to transmit
on a port before switching to the next in `balance-rr` mode.

### `resend_igmp` -- Resend IGMP

Number of IGMP membership reports to be sent after a failover.

### `num_grat_arp` -- Gratuitous ARP count

Number of gratuitous ARP packets to send after a failover. Must be equal
to `num_unsol_na` if both are defined.

### `num_unsol_na` -- Unsolicited NA count

Number of unsolicited IPv6 Neighbor Advertisements to send after a
failover. Same meaning in kernel as `num_grat_arp`.

### `peer_notif_delay` -- Peer notification delay

Delay in milliseconds between gratuitous ARP/NA notifications after a
failover.

### `tlb_dynamic_lb` -- TLB dynamic load balancing

Enables dynamic load balancing in `balance-tlb` mode. When `true`, the
bond rebalances traffic periodically.

### `ns_ip6_target` -- IPv6 neighbor solicitation targets

List of IPv6 addresses used as neighbor solicitation targets for IPv6 link
monitoring. Only valid when `arp_interval` is greater than 0.

## `ports` -- Bond ports

List of port configurations for the bond. When applying, if defined, it will
override the current port list.

Each port entry supports:

### `name` -- Port name

The interface name of the bond port. Mandatory.

### `priority` -- Port priority

Port priority for failover. Only valid in `active-backup`, `balance-tlb`, and
`balance-alb` modes.

### `queue-id` -- Port queue ID

The queue ID assigned to this port. Multiple ports sharing the same queue ID
is not supported by the Linux kernel.
