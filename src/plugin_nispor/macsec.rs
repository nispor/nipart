// SPDX-License-Identifier: Apache-2.0

use nipart::{BaseInterface, MacSecConfig, MacSecInterface, MacSecValidate};

fn np_mac_sec_validate_to_nipart(v: nispor::MacSecValidate) -> MacSecValidate {
    match v {
        nispor::MacSecValidate::Disabled => MacSecValidate::Disabled,
        nispor::MacSecValidate::Check => MacSecValidate::Check,
        nispor::MacSecValidate::Strict => MacSecValidate::Strict,
        _ => {
            log::warn!("Unknown MACsec validate mode {:?}", v);
            MacSecValidate::default()
        }
    }
}

pub(crate) fn np_macsec_to_nipart(
    np_iface: &nispor::Iface,
    base_iface: BaseInterface,
) -> MacSecInterface {
    let macsec_conf = np_iface.macsec.as_ref().map(|np_macsec_info| {
        let mut conf = MacSecConfig::default();
        conf.encrypt = np_macsec_info.encrypt;
        conf.port = np_macsec_info.port.into();
        conf.validation =
            np_mac_sec_validate_to_nipart(np_macsec_info.validate);
        conf.send_sci = np_macsec_info.send_sci;
        conf.base_iface = np_macsec_info.base_iface.clone().unwrap_or_default();
        conf.mka_cak = None;
        conf.mka_ckn = None;
        conf
    });

    let mut ret = MacSecInterface::default();
    ret.base = base_iface;
    ret.macsec = macsec_conf;
    ret
}
