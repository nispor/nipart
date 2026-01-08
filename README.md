# Nipart

Nipart is short of `Network Inspection Department` targeting to provides user
facing network management including:

 * Daemon for configuration management, plugin management and client
   communication.
 * Command line tools for user or script usage.
 * UNIX socket to daemon for API communication.

## Binaries and Libraries

 * `cli`: CLI tool for communicating with daemon -- `npt`
 * `daemon`: The daemon -- `nipartd`
 * `nipart`: Rust crate for daemon communication and daemon free actions
 * `python-nipart`: Python API for daemon communication
 * `plugin-demo`: Demonstration on how to create plugin for nipart

## Features
 * No Daemon mode (apply and quit)
 * Simply plugin design
 * Native support of [Nmstate][nmstate_url] schema

## Run Server

```bash
env CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='sudo' \
    cargo run --bin nipartd
```

## Run Client

```bash
env CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='sudo' \
    cargo run --bin npt
```

[nmstate_url]: https://nmstate.io/
