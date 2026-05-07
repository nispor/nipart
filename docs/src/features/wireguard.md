<!-- vim-markdown-toc GFM -->

* [Wireguard](#wireguard)
    * [`private-key` -- Private key](#private-key----private-key)
    * [`public-key` -- Public key](#public-key----public-key)
    * [`listen-port` -- Listen port](#listen-port----listen-port)
    * [`fwmark` -- Firewall mark](#fwmark----firewall-mark)
    * [`peers` -- Peer configurations](#peers----peer-configurations)
        * [`endpoint` -- Peer endpoint](#endpoint----peer-endpoint)
        * [`public-key` -- Peer public key](#public-key----peer-public-key)
        * [`preshared-key` -- Preshared key](#preshared-key----preshared-key)
        * [`persistent-keepalive` -- Persistent keepalive](#persistent-keepalive----persistent-keepalive)
        * [`allowed-ips` -- Allowed IPs](#allowed-ips----allowed-ips)
        * [`protocol-version` -- Protocol version](#protocol-version----protocol-version)
        * [`last-handshake` -- Last handshake time](#last-handshake----last-handshake-time)
        * [`rx-bytes` -- Received bytes](#rx-bytes----received-bytes)
        * [`tx-bytes` -- Transmitted bytes](#tx-bytes----transmitted-bytes)

<!-- vim-markdown-toc -->
# Wireguard

Example YAML of WireGuard interface configuration:

```yaml
version: 1
interfaces:
- name: wg0
  type: wireguard
  state: up
  wireguard:
    private-key: xH4dTz3dN3LzP2gE2kR8pA7sV9cF0bN1mQ5wY6uJ8k=
    listen-port: 51820
    fwmark: 0
    peers:
    - endpoint: 192.0.2.1:51820
      public-key: r3V5cF0bN1mQ5wY6uJ8k=xH4dTz3dN3LzP2gE2kR8pA7sV9=
      preshared-key: p7sV9cF0bN1mQ5wY6uJ8k=xH4dTz3dN3LzP2gE2kR8pA=
      persistent-keepalive: 25
      allowed-ips:
      - ip: 10.0.0.0
        prefix-length: 24
      - ip: 192.168.0.0
        prefix-length: 16
```

## `private-key` -- Private key

Base64 encoded private key. Required when creating a new WireGuard interface.
Will be displayed as `<_hidden_>` in debug/display output.

Set to `<_hidden_>` when applying to an existing interface to keep the current
private key unchanged.

## `public-key` -- Public key

Base64 encoded public key. Query only property, ignored when applying.

## `listen-port` -- Listen port

The UDP port to listen on for incoming connections. If not defined, the kernel
will choose a random port.

## `fwmark` -- Firewall mark

The firewall mark (fwmark) value for outgoing packets.

## `peers` -- Peer configurations

List of peer configurations. If defined, overrides the existing peer list. If
undefined, preserves current peers.

### `endpoint` -- Peer endpoint

The endpoint address and port of the peer in `ip:port` format, e.g.
`192.0.2.1:51820`. Mandatory for each peer configuration.

### `public-key` -- Peer public key

Base64 encoded public key of the peer. Used to identify the peer.

### `preshared-key` -- Preshared key

Base64 encoded preshared key for additional security via symmetric key
encryption. Displayed as `<_hidden_>` in debug/display output.

Set to `<_hidden_>` when applying to an existing interface to keep the current
preshared key unchanged.

### `persistent-keepalive` -- Persistent keepalive

The interval in seconds between keepalive packets. Used to maintain NAT/bridge
mappings.

### `allowed-ips` -- Allowed IPs

List of IP prefixes allowed for this peer. Each entry contains:
 * `ip`: The IP address.
 * `prefix-length`: The prefix length (CIDR mask).

### `protocol-version` -- Protocol version

The WireGuard protocol version.

### `last-handshake` -- Last handshake time

Query only property. Shows the time since the last handshake (e.g.
`32 seconds ago`).

### `rx-bytes` -- Received bytes

Query only property. Total bytes received from this peer.

### `tx-bytes` -- Transmitted bytes

Query only property. Total bytes transmitted to this peer.
