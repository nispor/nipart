# Nipart

**Work in Progress Slowly**

Nipart is short of `Network Inspection Department` targeting to provides user
facing network management including:
 * Daemon for plugin management and client communication.
 * Plugins for network configuration, configuration file manipulation and
   decision making.
 * Command line tools for user or script usage.
 * UNIX socket to daemon for API usage.

## Goal

* Easy for user to configure the network to fit their complex needs.
* Easy for developer to contribute.

## HOWTO

### Daemon

```sh
cargo build; cargo run --bin nipartd
```

### CLI

```sh
cargo run --bin nipc query eth1
```

## Design
* Nispor providing __full__ network status of kernel for all interfaces
  or single interface.
* Nispor taking __single__ interface __full__ configuration to apply.
* Nipart provides:
    * Point in time network state query, no caching.
    * Unified network state both from user space and kernel, so that
      user could get full picture of certain interface.
    * Smart network management with minimum prerequisite knowledge.
        * Ordering the network creation/management to kernel/user space.
        * Wraping complex layout to simple options.
    * Front-end plugins for variable developer friendly APIs.
        * NetworkManager daemon only provide unix socket interface
          for querying, changing and notification.
        * Frondends(DBUS, varlink, etc) just wrapping unix socket to their
          own tech.
        * Easy to creating binding for other develop lanagurages.
    * Plugins for DHCP, 802.1x, OVS, VPN, WIFI, DNS and etc.
        * Socket communication to child process of plugin allowing
          plugin been written in any language it likes.
        * A dedicate plugin for backwards compatibility.


## What should nipart core do and not do

* `nipartd` -- daemon
    * Provide plugin management.
    * Provide socket for plugin and API communication.
    * Do not need to understand the detailed schema of each interface.
    * Do not need to verify whether plugin is doing its works.
    * Do not maintain cache.

* `libnipart` -- API for communication to daemon

* `nipc` -- CLI tools

## What should nipart plugins do and not do

 * Configure plugin handle configuration saving/loading/converting.

 * Network apply plugin handle:
    * Network state querying.
    * Configuration validation.
    * Network configuration applying.

 * Command plugin for converting user request or network events to serial
   of specific commands to other plugins. For example, command plugin
   can invoke script before or after interface activation done(DHCP succeeded).


## TODO

* Tests
    * IPC request big size at daemon
    * 1000+ concurrent IPC requests to daemon, daemon should queue them
      instead of allocate big CPU/memeory to finish them concurrently.
    * IPC timeout
* Daemon for unix socket commands
* Plugin interface
* Handle concurrency on changing state.
* ABI Backwards compatibility:
    * set default value if unknown when serialize and deserialize
    * Use function of struct/enum instead of itself.
* Split `lib/nipart` to `lib/nipart` and `lib/nipart_plugin`.
