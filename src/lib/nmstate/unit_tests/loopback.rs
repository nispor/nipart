// SPDX-License-Identifier: Apache-2.0

use crate::{Interface, MergedNetworkState, NetworkState};

#[test]
fn test_loopback_include_default_ip() {
    let desired: NetworkState = serde_yaml::from_str(
        r#"
        version: 1
        interfaces:
        - name: lo
          type: loopback
          ipv4:
            enabled: true
            address:
            - ip: 127.0.0.2
              prefix-length: 32
          ipv6:
            enabled: true
            address:
            - ip: ::2
              prefix-length: 128
        "#,
    )
    .unwrap();

    let merged = MergedNetworkState::new(
        desired,
        NetworkState::default(),
        Default::default(),
    )
    .unwrap();

    let apply_state = merged.gen_state_for_apply();
    let apply_iface = apply_state.ifaces.kernel_ifaces.get("lo").unwrap();

    let expected: Interface = serde_yaml::from_str(
        r#"
        name: lo
        type: loopback
        ipv4:
          enabled: true
          address:
          - ip: 127.0.0.2
            prefix-length: 32
          - ip: 127.0.0.1
            prefix-length: 8
        ipv6:
          enabled: true
          address:
          - ip: ::2
            prefix-length: 128
          - ip: ::1
            prefix-length: 128
        "#,
    )
    .unwrap();

    assert_eq!(apply_iface, &expected);
}
