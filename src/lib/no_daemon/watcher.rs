// SPDX-License-Identifier: Apache-2.0

use futures_util::stream::{StreamExt, TryStreamExt};
use rtnetlink::{
    MulticastGroup, new_multicast_connection,
    packet_core::NetlinkPayload,
    packet_route::{
        RouteNetlinkMessage,
        link::{LinkAttribute, State},
    },
};

use crate::{ErrorKind, NipartError, NipartNoDaemon};

impl NipartNoDaemon {
    pub async fn wait_link_carrier_up(
        iface_name: &str,
    ) -> Result<(), NipartError> {
        wait_link_carrier(iface_name, true).await
    }

    pub async fn wait_link_carrier_down(
        iface_name: &str,
    ) -> Result<(), NipartError> {
        wait_link_carrier(iface_name, false).await
    }
}

async fn wait_link_carrier(
    iface_name: &str,
    link_up: bool,
) -> Result<(), NipartError> {
    // netlink multicast socket will be used for one-time query and also follow
    // up monitor
    let (conn, handle, mut messages) =
        new_multicast_connection(&[MulticastGroup::Link]).map_err(|e| {
            NipartError::new(
                ErrorKind::InvalidArgument,
                format!(
                    "Failed to create netlink multicast socket for interface \
                     {iface_name}: {e}"
                ),
            )
        })?;
    tokio::spawn(conn);

    let cur_link_state = is_link_carrier_up(&handle, iface_name).await?;
    if link_up == cur_link_state {
        return Ok(());
    }

    let iface_name_attr = LinkAttribute::IfName(iface_name.to_string());

    while let Some((nl_msg, _)) = messages.next().await {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewLink(
            link_msg,
        )) = nl_msg.payload
            && link_msg
                .attributes
                .iter()
                .any(|attr| attr == &iface_name_attr)
                && link_msg.attributes.iter().any(|attr| {
                    if link_up {
                        &LinkAttribute::OperState(State::Up) == attr
                    } else {
                        &LinkAttribute::OperState(State::Up) != attr
                    }
                })
            {
                return Ok(());
            }
    }
    Err(NipartError::new(
        ErrorKind::Bug,
        "wait_link_carrier(): Kernel terminated the netlink multicast socket \
         connection"
            .into(),
    ))
}

async fn is_link_carrier_up(
    handle: &rtnetlink::Handle,
    iface_name: &str,
) -> Result<bool, NipartError> {
    let mut links = handle
        .link()
        .get()
        .match_name(iface_name.to_string())
        .execute();
    while let Some(link_msg) = links.try_next().await.map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!(
                "Failed to query rtnetlink link subsystem for checking link \
                 carrier of {}: {e}",
                iface_name
            ),
        )
    })? {
        for attr in link_msg.attributes {
            if LinkAttribute::OperState(State::Up) == attr {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
