# Title: ADR-005 Native Plugin vs External Plugin

## Status: Proposed

## Context

Placing plugins used for all use cases as thread to daemon brings:
 * Less external process
 * Quicker inter-threads communication
 * No serialize/deserialize overheads.

All need clear guideline on what kind of plugins could be run as thread of
daemon.

Whether plugin running as thread of daemon or standalone process, there
should be no difference on what this plugin could do. (e.g. External DHCP
plugin should receive identical event as internal DHCP plugin.)

## Decision

Native Plugin:
 * Invoked as thread of daemon.
 * Communicate with daemon through two pairs of MPSC channels holding
   `NipartEvent`.
 * Maintained by developers of nipart.
 * Provide essential functionally widely used by most use cases.
 * Voted by admin group of nipart.
 * Is rust library crate, daemon is using `NipartPluginNispor::handle_event()`.

External Plugin:
 * Invoked as standalone thread.
 * Listening on native abstract(path leading with `\0`) UNIX socket.
   Receiving `NipartEvent` from daemon.
 * Sending `NipartEvent` to daemon through daemon public API.
 * Provide optional functionally. (e.g. OpenvSwitch, D-BUS, WiFi)
 * Is rust binary crate, main function is using `NipartPluginNispor::run()`.

Both Native plugin and External plugin is required to implement the same Rust
Trait `NipartPlugin`.

To ensure the consistency of `NipartEvent` regardless it is transferred through
MPSC or IPC, any property added to data type used by `NipartEvent` should have
unit test proving no data lose during serialization and deserialization.

## Consequences

### Better

 * Single binary of daemon is enough to provide full support of common use
   cases.
 * MPSC communication is faster and simpler than IPC.

### Worse

 * Since Native plugin is communicate without serialization, extra care
   required to make sure no information is lost during
   serialization/deserialization of `NipartEvent`, otherwise, external plugin
   cannot function the same as native plugin.
