# Title: ADR-001: API Design

## Status: Accepted

## Context

The VPN plugin interface of NetworkManager 1.x is a subset of full network
configure interface. This has made it impossible to implement OpenvSwitch
feature as plugin of NetworkManager, but need to be loaded by main daemon
from so file.

The nispor and nm folder of Nmstate 2.x rust code also use many
`pub(crate)` functions which limit our effort on module isolation.

With full feature plugin interface, nipart daemon could focusing on
plugin management and event exchange while plugin focusing on network
configurations with clear boundary on who should do what.

## Decision

The nipart daemon should only provides:
 * User API through UNIX file based stream socket
 * Plugin management
 * Message exchange between plugins and users
 * Convert user request to several plugin events
 * Provide rust `Trait` to simplify the plugin development

The nipart plugin should:
 * Running as standalone process listening on UNIX abstract stream socket with
   path provided as ARGV `$1`
 * Plugins are isolated to each other, can only communicate each other through
   daemon
 * All network configuration should be done by plugin not daemon

## Consequences

### Better

 * Better isolation, small code base on each components
 * Simplify plugin development

### Worse

 * IPC communication require more effort than threads communication
 * ASYNC design is more complex and not friendly for new contributor

All can be solved by cautious design, plugin developer is not required to
understand the plugin interface internal implementation.
