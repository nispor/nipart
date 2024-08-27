// SPDX-License-Identifier: Apache-2.0

use crate::{
    NipartEvent, NipartEventAddress, NipartLogLevel, NipartPluginEvent,
    NipartPluginInfo, NipartUserEvent,
};

pub(crate) fn handle_query_plugin_info(
    uuid: u128,
    src: &NipartEventAddress,
    plugin_info: NipartPluginInfo,
    plugin_name: &str,
) -> NipartEvent {
    log::debug!("Querying plugin info of {}", plugin_name);
    NipartEvent::new_with_uuid(
        uuid,
        NipartUserEvent::None,
        NipartPluginEvent::QueryPluginInfoReply(plugin_info),
        NipartEventAddress::Unicast(plugin_name.to_string()),
        src.clone(),
        crate::DEFAULT_TIMEOUT,
    )
}

pub(crate) fn handle_change_log_level(
    log_level: NipartLogLevel,
    uuid: u128,
    plugin_name: &str,
) -> NipartEvent {
    log::debug!("Setting log level of {} to {log_level}", plugin_name);
    log::set_max_level(log_level.into());
    NipartEvent::new_with_uuid(
        uuid,
        NipartUserEvent::None,
        NipartPluginEvent::QueryLogLevelReply(log_level),
        NipartEventAddress::Unicast(plugin_name.to_string()),
        NipartEventAddress::Commander,
        crate::DEFAULT_TIMEOUT,
    )
}

pub(crate) fn handle_query_log_level(
    uuid: u128,
    plugin_name: &str,
) -> NipartEvent {
    log::debug!("Querying log level of {}", plugin_name);
    NipartEvent::new_with_uuid(
        uuid,
        NipartUserEvent::None,
        NipartPluginEvent::QueryLogLevelReply(log::max_level().into()),
        NipartEventAddress::Unicast(plugin_name.to_string()),
        NipartEventAddress::Commander,
        crate::DEFAULT_TIMEOUT,
    )
}
