<!-- vim-markdown-toc GFM -->

* [Code Layout](#code-layout)
    * [Daemon](#daemon)
        * [Daemon Start up](#daemon-start-up)
        * [Daemon User API Thread](#daemon-user-api-thread)
        * [Daemon Switch Thread](#daemon-switch-thread)
    * [Nispor Plugin -- Kernel Query and Config](#nispor-plugin----kernel-query-and-config)
    * [Mallory Plugin -- State](#mallory-plugin----state)
    * [BaiZe Plugin -- Monitor](#baize-plugin----monitor)
    * [Mozim Plugin -- DHCP](#mozim-plugin----dhcp)
    * [OVS Plugin](#ovs-plugin)
    * [LLDP Plugin](#lldp-plugin)
    * [Librarian Plugin -- Conf](#librarian-plugin----conf)

<!-- vim-markdown-toc -->

# Code Layout

## Daemon

### Daemon Start up

 * Daemon invokes plugins base on config.
 * Daemon start API thread listening on public API UNIX socket.
 * Daemon start commander thread converting API event to/from plugin events.
 * Daemon start switch thread forwarding events between api thread, commander
   thread and plugins.

### Daemon User API Thread

 * The API thread invoke thread all each `NipartConnection`
 * The `NipartEvent` received from user will changed the target to commander,
   and send to switch thread.
 * When switch send back event to API thread, search on which
   `NipartConnection` should used to send using a tracking queue
   shared between sub-threads of API thread.

### Daemon Switch Thread

 * Forward the `NipartEvent` base on its source and designation.
 * The switch holds multiple tokio mpsc channels to daemon API thread and
   commander thread.
 * The switch holds `NipartConnection` to plugin socket.

## Nispor Plugin -- Kernel Query and Config

## Mallory Plugin -- State

Mallory is name of `Man-in-the-middle`. In our case, it is a plugin alter the
network state:

 * Validate user input state and sanitize
 * Convert state into state ready for other plugins to apply.
 * Merge state from plugins and report to user.

## BaiZe Plugin -- Monitor

The [`Bai Ze`][baize_wiki] is a mythical beast in ancient China.  It can speak,
understand the feelings of all things.
(白泽神兽，能言，达于万物之情)

The monitor plugin should send `NipartEvent` to commander when event of
`NipartMonitorRule` happens.

## Mozim Plugin -- DHCP

## OVS Plugin

## LLDP Plugin

## Librarian Plugin -- Conf

Librarian to keep the configurations.

[baize_wiki]: https://en.wikipedia.org/wiki/Bai_Ze
