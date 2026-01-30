// SPDX-License-Identifier: Apache-2.0

use crate::{
    BaseInterface, ErrorKind, NipartError, VlanConfig, VlanInterface,
    VlanProtocol, VlanQosMapping, VlanRegistrationProtocol,
};

impl From<nispor::VlanProtocol> for VlanProtocol {
    fn from(v: nispor::VlanProtocol) -> Self {
        match v {
            nispor::VlanProtocol::Ieee8021Q => VlanProtocol::Ieee8021Q,
            nispor::VlanProtocol::Ieee8021AD => VlanProtocol::Ieee8021Ad,
            p => {
                log::debug!("Got unknown VLAN protocol {p:?}");
                VlanProtocol::Unknown
            }
        }
    }
}

impl From<VlanProtocol> for nispor::VlanProtocol {
    fn from(v: VlanProtocol) -> Self {
        match v {
            VlanProtocol::Ieee8021Q => Self::Ieee8021Q,
            VlanProtocol::Ieee8021Ad => Self::Ieee8021AD,
            VlanProtocol::Unknown => {
                log::debug!("Unknown VLAN protocol {v}, treating as 802.1q");
                Self::Ieee8021Q
            }
        }
    }
}

impl From<&VlanQosMapping> for nispor::VlanQosMapping {
    fn from(v: &VlanQosMapping) -> Self {
        Self {
            from: v.from,
            to: v.to,
        }
    }
}

impl From<&VlanConfig> for nispor::VlanConf {
    fn from(v: &VlanConfig) -> Self {
        let mut np_vlan = nispor::VlanConf::default();
        np_vlan.vlan_id = v.id;
        np_vlan.base_iface = v.base_iface.clone();
        np_vlan.protocol = v.protocol.map(|v| v.into());
        if let Some(protocol) = v.registration_protocol {
            match protocol {
                VlanRegistrationProtocol::Gvrp => {
                    np_vlan.is_gvrp = Some(true);
                    np_vlan.is_mvrp = Some(false);
                }
                VlanRegistrationProtocol::Mvrp => {
                    np_vlan.is_gvrp = Some(false);
                    np_vlan.is_mvrp = Some(true);
                }
                VlanRegistrationProtocol::None => {
                    np_vlan.is_gvrp = Some(false);
                    np_vlan.is_mvrp = Some(false);
                }
            }
        }

        if let Some(v) = v.reorder_headers {
            np_vlan.is_reorder_hdr = Some(v)
        }
        if let Some(v) = v.loose_binding {
            np_vlan.is_loose_binding = Some(v)
        }
        if let Some(v) = v.bridge_binding {
            np_vlan.is_bridge_binding = Some(v)
        }
        if let Some(qoses) = v.ingress_qos_map.as_ref() {
            np_vlan.ingress_qos_map =
                Some(qoses.iter().map(nispor::VlanQosMapping::from).collect());
        }
        if let Some(qoses) = v.egress_qos_map.as_ref() {
            np_vlan.egress_qos_map =
                Some(qoses.iter().map(nispor::VlanQosMapping::from).collect());
        }
        np_vlan
    }
}

impl From<&nispor::VlanInfo> for VlanConfig {
    fn from(np_vlan_conf: &nispor::VlanInfo) -> Self {
        VlanConfig {
            id: Some(np_vlan_conf.vlan_id),
            base_iface: Some(np_vlan_conf.base_iface.clone()),
            protocol: Some(np_vlan_conf.protocol.into()),
            reorder_headers: Some(np_vlan_conf.is_reorder_hdr),
            loose_binding: Some(np_vlan_conf.is_loose_binding),
            bridge_binding: Some(np_vlan_conf.is_bridge_binding),
            // They are mutually exclusive, vlan cannot be gvrp and mvrp
            // at the same time
            registration_protocol: if np_vlan_conf.is_gvrp {
                Some(VlanRegistrationProtocol::Gvrp)
            } else if np_vlan_conf.is_mvrp {
                Some(VlanRegistrationProtocol::Mvrp)
            } else {
                Some(VlanRegistrationProtocol::None)
            },
            ingress_qos_map: get_qos_map(&np_vlan_conf.ingress_qos_map),
            egress_qos_map: get_qos_map(&np_vlan_conf.egress_qos_map),
        }
    }
}

pub(crate) fn apply_vlan_conf(
    mut np_iface: nispor::IfaceConf,
    iface: &VlanInterface,
) -> Result<Vec<nispor::IfaceConf>, NipartError> {
    if let Some(vlan_conf) = iface.vlan.as_ref() {
        if vlan_conf.id.is_some() && vlan_conf.base_iface.as_ref().is_some() {
            np_iface.vlan = Some(vlan_conf.into());
        } else {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "apply_vlan_conf() got VLAN without ID or base-iface: \
                     {iface:?}"
                ),
            ));
        }
    }
    Ok(vec![np_iface])
}

impl VlanInterface {
    pub(crate) fn new_from_nispor(
        base_iface: BaseInterface,
        np_iface: &nispor::Iface,
    ) -> Self {
        if let Some(np_vlan_conf) = np_iface.vlan.as_ref() {
            Self {
                base: base_iface,
                vlan: Some(np_vlan_conf.into()),
            }
        } else {
            Self {
                base: base_iface,
                ..Default::default()
            }
        }
    }
}

fn get_qos_map(
    np_maps: &[nispor::VlanQosMapping],
) -> Option<Vec<crate::VlanQosMapping>> {
    if np_maps.is_empty() {
        None
    } else {
        let mut ret = Vec::new();
        for np_map in np_maps {
            ret.push(crate::VlanQosMapping {
                from: np_map.from,
                to: np_map.to,
            });
        }
        ret.sort_unstable();
        Some(ret)
    }
}
