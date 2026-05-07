<!-- vim-markdown-toc GFM -->

* [IP Address](#ip-address)
    * [`enabled` -- Enable IP stack](#enabled----enable-ip-stack)
    * [`dhcp` -- Enable DHCP](#dhcp----enable-dhcp)
    * [`address` -- IP address configuration](#address----ip-address-configuration)
    * [`autoconf` -- Enable IPv6 autoconfiguration](#autoconf----enable-ipv6-autoconfiguration)

<!-- vim-markdown-toc -->

# IP Address

Example YAML of static IP address configuration:

```yaml
version: 1
interfaces:
- name: eth1
  type: veth
  state: up
  ipv4:
    enabled: true
    dhcp: false
    address:
    - ip: 192.0.2.252
      prefix-length: 24
    - ip: 192.0.2.251
      prefix-length: 24
  ipv6:
    enabled: true
    dhcp: false
    autoconf: false
    address:
    - ip: 2001:db8:1::1
      prefix-length: 64
    - ip: 2001:db8:2::1
      prefix-length: 64
```


## `enabled` -- Enable IP stack

When set to `false`, the IP stack will be disabled for the interface. This
means that the interface will not be able to send or receive IP packets.

## `dhcp` -- Enable DHCP

When set to `true`, the interface will attempt to obtain an IP address via
DHCP. This is only applicable if `enabled` is set to `true`.

**NOTE**: DHCPv6 cannot set IPv6 route. Use `autoconf` to obtain IPv6 address
and route via IPv6 Route Advertisement instead.

## `address` -- IP address configuration

The `address` field is a list of IP address configurations for the interface.

Each entry in the list should contain the following fields:
- `ip`: The IP address to assign to the interface.
- `prefix-length`: The prefix length (subnet mask) for the IP address.
- `valid-life-time`: (Optional) The valid lifetime of the IP address in
  seconds. If not specified, the IP address will be considered as static IP and
  valid indefinitely.
- `preferred-life-time`: (Optional) The preferred lifetime of the IP address in
  seconds. If not specified, the IP address will be considered as static IP and
  preferred indefinitely.

## `autoconf` -- Enable IPv6 autoconfiguration

When set to `true`, the interface will attempt to obtain an IPv6 address via
IPv6 Route Advertisement. This is only applicable if `enabled` is set to
`true`.
