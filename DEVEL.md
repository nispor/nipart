<!-- vim-markdown-toc GFM -->

* [Main daemon](#main-daemon)
* [IPC](#ipc)
* [Work flow](#work-flow)
    * [Daemon Start up](#daemon-start-up)
    * [User API request](#user-api-request)
    * [External event happens](#external-event-happens)
* [NipartEvent](#nipartevent)
* [Cody Plugin -- Commander](#cody-plugin----commander)
* [Argus Plugin -- Monitor](#argus-plugin----monitor)
    * [Receive Event](#receive-event)
    * [Action](#action)
    * [Output Event](#output-event)
* [Nispor Plugin -- Kernel](#nispor-plugin----kernel)
* [Mallory Plugin -- State](#mallory-plugin----state)
* [Mozim Plugin -- DHCP](#mozim-plugin----dhcp)
    * [Receive Event](#receive-event-1)
    * [Output Event](#output-event-1)
* [OVS Plugin](#ovs-plugin)
* [LLDP Plugin](#lldp-plugin)
* [Librarian Plugin -- Conf](#librarian-plugin----conf)

<!-- vim-markdown-toc -->

## Main daemon

 * Invoke plugins
 * Provide public API via unix socket
 * NipartEvent switch between plugins.

## IPC

 * Async
 * Safe size
 * Public IPC command to NipartEvent
 * Plugin

## Work flow

### Daemon Start up

 * Daemon invokes plugins base on config.
 * Daemon listen on public API UNIX socket.
 * The config plugin will read network config and generate `NipartMonitorRull`
   with network config attached.
 * The monitor plugin will generate NipartEvent when monitored link up.

### User API request

 * Daemon got Public IPC request as `NipartIpc::ConnectionAdd`.
 * Daemon send IPC request to the commander plugin.
 * The commander plugin convert IPC request to a set of `NipartEvent` and
   send back to daemon.
 * Daemon redirect these events to correct plugins.

### External event happens

 * The monitor plugin generate NipartEvent when matching external event
   happens.
 * The daemon redirect NipartEvent to the commander plugin.
 * The commander plugin convert NipartEvent into a set of NipartEvent and
   send out.

## NipartEvent

```
struct NipartEvent {
    uuid: String,
    ref_uuid: Option<String>,
    state: Todo|Ongoing|Cancled|Done,
    reciver: commander | daemon | dhcp | kernel | ovs | lldp | monitor |
             state | config
    data : enum NipartEventData {
    }
}
```

## Cody Plugin -- Commander

 * Convert NipartEvent to a set of NipartEvent

## Argus Plugin -- Monitor

Monitor network state and generate NipartEvent base on requested
`NipartMonitorRull`.

### Receive Event

```
struct NipartEvent {
    state: Todo,
    reciver: monitor,
    data: NipartEventData::MonitorRull(
        NipartMonitorRull::Link( NipartMonitorRullLink {
            iface_index:  9,
            interested: Up,
            // What event should be emitted when it happens
            followup: NipartEvent,
        })
    )
}
```

### Action

Hook on netlink socket and register for monitor, only generate NipartEvent
when link state __changed__.

### Output Event

```
struct NipartEvent {
    state: Todo,
    reciver: commander,
    data: NipartEventData::LinkUp(
        NipartEventLinkUp {
            iface_index:  9,
            iface_name: "eth1",
        })
    )
}
```

## Nispor Plugin -- Kernel
 * Query kernel network state
 * Apply kernel network state

## Mallory Plugin -- State

 * Validate user input state and sanitize
 * Convert state into state ready for other plugins to apply.

## Mozim Plugin -- DHCP

### Receive Event

```
struct NipartEvent {
    state: Todo,
    reciver: dhcp,
    data: NipartEventData::StartDhcp(
        NipartDhcpConfig {
            iface_index:  9,
            ... // other Dhcp configs
        }
    )
}
```

### Output Event

When lease acquired or updated, emit

```
struct NipartEvent {
    state: Todo,
    reciver: kernel,
    data: NipartEventData::DhcpLeaseUpdate(
        NipartDhcpLease {
            iface_index:  9,
            iface_name: "eth1",
            ipv4_addr: "192.0.2.101",
            ... // other DHCP lease config
        }
    )
}
```

## OVS Plugin

 *

## LLDP Plugin

 *

## Librarian Plugin -- Conf

Persisting the config
