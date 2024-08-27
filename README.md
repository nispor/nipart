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
cargo build;
env CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='sudo -E' \
    cargo run --bin nipartd
```

### CLI

```sh
env CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='sudo -E' \
    cargo run --bin nipc
```
