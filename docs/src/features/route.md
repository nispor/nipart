<!-- vim-markdown-toc GFM -->

* [Route](#route)
    * [`running` -- Current kernel routes](#running----current-kernel-routes)
    * [`config` -- Desired static routes](#config----desired-static-routes)
    * [`state` -- Route state](#state----route-state)
    * [`destination` -- Route destination network](#destination----route-destination-network)
    * [`next-hop-interface` -- Next hop interface](#next-hop-interface----next-hop-interface)
    * [`next-hop-address` -- Next hop IP address](#next-hop-address----next-hop-ip-address)
    * [`metric` -- Route metric](#metric----route-metric)
    * [`table-id` -- Route table ID](#table-id----route-table-id)
    * [`weight` -- ECMP route weight](#weight----ecmp-route-weight)
    * [`route-type` -- Route type](#route-type----route-type)
    * [`source` -- Source address](#source----source-address)
    * [`cwnd` -- Congestion window clamp](#cwnd----congestion-window-clamp)
    * [`initcwnd` -- Initial congestion window](#initcwnd----initial-congestion-window)
    * [`initrwnd` -- Initial receive window](#initrwnd----initial-receive-window)
    * [`mtu` -- Route MTU](#mtu----route-mtu)
    * [`quickack` -- Quick ACK](#quickack----quick-ack)
    * [`advmss` -- Advertised MSS](#advmss----advertised-mss)

<!-- vim-markdown-toc -->
# Route

Example YAML of route configuration:

```yaml
version: 1
routes:
  config:
  - destination: 0.0.0.0/0
    next-hop-interface: eth1
    next-hop-address: 192.0.2.1
    metric: 100
    table-id: 254
  - destination: 0.0.0.0/0
    next-hop-interface: eth1
    next-hop-address: 192.0.2.2
    metric: 100
    weight: 2
  - destination: 2001:db8::/64
    next-hop-interface: eth1
    next-hop-address: 2001:db8::1
  - destination: 10.0.0.0/24
    next-hop-interface: eth1
    next-hop-address: 192.0.2.254
  - state: absent
    next-hop-interface: eth1
```

## `running` -- Current kernel routes

Query only property. Contains the currently active routes from kernel, filtered
to routes with universe or link scope and only from these protocols: `boot`,
`static`, `ra`, `dhcp`, `mrouted`, `keepalived`, `babel`.

Ignored when applying.

## `config` -- Desired static routes

The desired static routes. Contains routes from universe or link scope, and only
from protocols `boot` and `static`.

When applying, `None` means preserve current routes. This property is not
overriding but adding specified routes to existing routes. To delete a route
entry, set `state` to `absent`. Any property of an absent `RouteEntry` set to
`None` means wildcard matching. For example, this will remove all routes next
hop to interface `eth1`:

```yaml
routes:
  config:
  - next-hop-interface: eth1
    state: absent
```

To change a route entry, you need to delete the old one and add the new one
(can be in single transaction).

## `state` -- Route state

Used only for deleting routes when applying:
 * `absent`: Mark a route entry to be removed. Properties set to `None` act as
   wildcards for matching which routes to delete.
 * `ignore`: Mark a route as not managed by nipart.

## `destination` -- Route destination network

The destination network of the route in CIDR notation, e.g. `0.0.0.0/0` for
default gateway or `10.0.0.0/24`.

Mandatory for every non-absent route.

`0.0.0.0/8` and its subnet cannot be used as the route destination for unicast
route. Use `0.0.0.0/0` for default gateway instead.

## `next-hop-interface` -- Next hop interface

The interface name of the next hop, e.g. `eth1`.

Mandatory for every non-absent unicast route. Not required for routes with
route type `Blackhole`, `Unreachable`, or `Prohibit`.

## `next-hop-address` -- Next hop IP address

The IP address of the next hop router, e.g. `192.0.2.1`.

Optional. When set to empty string for absent route, it will only delete routes
without a `next-hop-address`.

## `metric` -- Route metric

The route metric (priority). Default is backend-defined. Lower metric is
preferred.

## `table-id` -- Route table ID

The routing table ID. Default is `254` (main routing table). Set to `0` to use
backend default.

## `weight` -- ECMP route weight

The weight for Equal-Cost Multi-Path (ECMP) routing. Valid range is 1 to 256.

When multiple route entries share the same `destination` and `metric` but have
different `next-hop-address`, they form ECMP routes. Kernel distributes traffic
across them proportionally to the weight.

IPv6 ECMP route with weight is not supported yet.

## `route-type` -- Route type

The type of route:
 * `blackhole`: Packets matching this route are silently discarded.
 * `unreachable`: Packets matching this route generate an ICMP unreachable
   message.
 * `prohibit`: Packets matching this route generate an ICMP administratively
   prohibited message.

A route without `route-type` is a unicast route (default).

A non-unicast route cannot have a `next-hop-interface` (except `lo`) or
`next-hop-address`.

## `source` -- Source address

The preferred source address for packets routed via this route. Specifies which
local IP address should be used as the source for outgoing packets matching
this route.

## `cwnd` -- Congestion window clamp

The congestion window clamp size in bytes. Cannot be set to 0.

## `initcwnd` -- Initial congestion window

The initial congestion window size in bytes (TCP initcwnd).

## `initrwnd` -- Initial receive window

The initial receive window size in bytes (TCP initrwnd).

## `mtu` -- Route MTU

The MTU of the route in bytes. Cannot be set to 0.

## `quickack` -- Quick ACK

When set to `true`, disables delayed TCP acknowledgments for connections using
this route.

## `advmss` -- Advertised MSS

The Maximum Segment Size (MSS) to advertise for TCP connections using this
route. Cannot be set to 0.
