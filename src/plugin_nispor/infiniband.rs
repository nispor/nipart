// SPDX-License-Identifier: Apache-2.0

use nipart::{
    BaseInterface, InfiniBandConfig, InfiniBandInterface, InfiniBandMode,
};

fn np_ipoib_mode_to_nipart(m: nispor::IpoibMode) -> InfiniBandMode {
    match m {
        nispor::IpoibMode::Datagram => InfiniBandMode::Datagram,
        nispor::IpoibMode::Connected => InfiniBandMode::Connected,
        _ => {
            log::warn!("Unknown IP over IB mode {:?}", m);
            InfiniBandMode::default()
        }
    }
}

pub(crate) fn np_ib_to_nipart(
    np_iface: &nispor::Iface,
    base_iface: BaseInterface,
) -> InfiniBandInterface {
    let ib_conf = np_iface.ipoib.as_ref().map(|np_ib_info| {
        let mut config = InfiniBandConfig::default();
        config.mode = np_ipoib_mode_to_nipart(np_ib_info.mode);
        config.base_iface = np_ib_info.base_iface.clone();
        config.pkey = Some(np_ib_info.pkey);
        config
    });

    let mut ret = InfiniBandInterface::default();
    ret.base = base_iface;
    ret.ib = ib_conf;
    ret
}
