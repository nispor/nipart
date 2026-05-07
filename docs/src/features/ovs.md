<!-- vim-markdown-toc GFM -->

* [OpenvSwitch Bridge](#openvswitch-bridge)
    * [`bridge`](#bridge)
        * [`ports` -- Bridge ports](#ports----bridge-ports)
            * [`name` -- Port name](#name----port-name)
    * [Port interfaces](#port-interfaces)
    * [OVS internal interface](#ovs-internal-interface)
    * [Limitations](#limitations)

<!-- vim-markdown-toc -->

# OpenvSwitch Bridge

Example YAML of OVS bridge configuration:

```yaml
version: 1
interfaces:
- name: br0
  type: ovs-bridge
  state: up
  bridge:
    ports:
    - name: eth1
    - name: eth2
- name: br0
  type: ovs-interface
  state: up
  controller: br0
  ipv4:
    enabled: false
  ipv6:
    enabled: false
- name: eth1
  type: ovs-interface
  state: up
  controller: br0
- name: eth2
  type: ovs-interface
  state: up
  controller: br0
```

OVS bridge consists of three parts: the `ovs-bridge` interface itself, one
`ovs-interface` for the bridge internal interface, and `ovs-interface` entries
for each port.

## `bridge`

The OVS bridge configuration.

### `ports` -- Bridge ports

List of port names attached to the bridge. Each port entry contains:

#### `name` -- Port name

The interface name of the port attached to this OVS bridge. The corresponding
interface should be defined with `type: ovs-interface` and
`controller: <bridge-name>`.

## Port interfaces

Ports are defined as separate interfaces with `type: ovs-interface` and
`controller` set to the OVS bridge name. They inherit all base interface
properties including IP configuration.

## OVS internal interface

The bridge itself typically has one `ovs-interface` with the same name as the
bridge, serving as the bridge's internal interface with IP configuration.

## Limitations

* OVS Bond is not supported yet.
* OVS `patch` and `dpdk` interface types are not supported yet.
