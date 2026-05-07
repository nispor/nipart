<!-- vim-markdown-toc GFM -->

* [Wait Online](#wait-online)
    * [Configuration](#configuration)
    * [Behavior](#behavior)
    * [Valid Conditions](#valid-conditions)

<!-- vim-markdown-toc -->

# Wait Online

`npt wait-online` waits for `nipartd` to configure the network and reach an
`online` state. Used by `nipart-wait-online.service` for systemd
`network-online.target`.

## Configuration

```yaml
wait-online:
  timeout-sec: 30
  conditions:
  - gateway4
  - gateway6
```


`wait-online` does not use partial updating - if defined, it overrides any
previous configuration. To keep existing settings, use the previously saved
config.

## Behavior

* When all conditions are met, the daemon considers the network online and
  `npt wait-online` exits with code 0.
* On timeout, `npt wait-online` exits with code 124
  (matching `/usr/bin/timeout`).
* Once the daemon reaches the online state, it stops tracking subsequent
  network changes and does **not** re-check whether the conditions are still
  met.

## Valid Conditions

| Condition | Description |
|---|---|
| `saved-config-applied` | All saved configs applied (excl. conditional act.) |
| `gateway` | IPv4 or IPv6 gateway added |
| `gateway4` | IPv4 gateway added |
| `gateway6` | IPv6 gateway added |

