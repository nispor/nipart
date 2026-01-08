// SPDX-License-Identifier: Apache-2.0

use crate::{ErrorKind, NipartstateInterface, WifiPhyInterface};

#[test]
fn test_wifi_phy_hold_wifi_cfg_with_other_base_iface() {
    let mut iface: WifiPhyInterface = serde_yaml::from_str(
        r#"---
        name: wlan0
        type: wifi-phy
        wifi:
          ssid: Test-WIFI
          password: 12345678
          base-iface: wlan1
        "#,
    )
    .unwrap();

    let result = iface.sanitize(None);
    assert!(result.is_err());

    if let Err(e) = result {
        assert_eq!(e.kind(), ErrorKind::InvalidArgument);
        assert!(e.msg.contains("wlan1"));
    }
}
