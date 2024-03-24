// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use futures::stream::StreamExt;
use netlink_packet_core::{NetlinkMessage, NetlinkPayload};
use netlink_packet_route::{link::LinkAttribute, RouteNetlinkMessage};
use netlink_sys::AsyncSocket;
use nipart::{
    ErrorKind, NipartError, NipartEvent, NipartEventAddress,
    NipartLinkMonitorKind, NipartLinkMonitorRule, NipartMonitorEvent,
    NipartNativePlugin, NipartPluginEvent, NipartUserEvent,
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BaizeLinkMonitorCmd {
    AddLinkRule(NipartLinkMonitorRule),
    DelLinkRule(NipartLinkMonitorRule),
}

#[derive(Debug)]
pub(crate) struct BaizeLinkMonitor {
    thread_handler: JoinHandle<()>,
    to_daemon: Sender<NipartEvent>,
    to_monitor: Sender<BaizeLinkMonitorCmd>,
}

impl Drop for BaizeLinkMonitor {
    fn drop(&mut self) {
        self.thread_handler.abort();
    }
}

const MPSC_CHANNLE_SIZE: usize = 1000;

impl BaizeLinkMonitor {
    pub(crate) fn new(
        to_daemon: Sender<NipartEvent>,
    ) -> Result<Self, NipartError> {
        let (plugin_to_monitor_tx, plugin_to_monitor_rx) =
            tokio::sync::mpsc::channel::<BaizeLinkMonitorCmd>(
                MPSC_CHANNLE_SIZE,
            );
        let to_daemon_clone = to_daemon.clone();
        let thread_handler = tokio::task::spawn(async move {
            LinkMonitorThread::process(to_daemon_clone, plugin_to_monitor_rx)
                .await
        });

        Ok(Self {
            thread_handler,
            to_monitor: plugin_to_monitor_tx,
            to_daemon,
        })
    }

    pub(crate) async fn add_link_rule(
        &mut self,
        rule: NipartLinkMonitorRule,
    ) -> Result<(), NipartError> {
        let already_link_up = is_link_up(rule.iface.as_str()).await?;

        match rule.kind {
            NipartLinkMonitorKind::Up => {
                if already_link_up {
                    send_link_notify(&self.to_daemon, &rule).await?;
                    return Ok(());
                }
            }
            NipartLinkMonitorKind::Down => {
                if !already_link_up {
                    send_link_notify(&self.to_daemon, &rule).await?;
                    return Ok(());
                }
            }
            kind => {
                return Err(NipartError::new(
                    ErrorKind::Bug,
                    format!("Bug: Unknown NipartLinkMonitorKind {kind}"),
                ))
            }
        }

        self.to_monitor
            .send(BaizeLinkMonitorCmd::AddLinkRule(rule))
            .await
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to send command: add rule \
                        to monitor thread: {e}"
                    ),
                )
            })
    }

    pub(crate) async fn del_link_rule(
        &mut self,
        rule: NipartLinkMonitorRule,
    ) -> Result<(), NipartError> {
        self.to_monitor
            .send(BaizeLinkMonitorCmd::DelLinkRule(rule))
            .await
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to send command: del rule \
                        to monitor thread: {e}"
                    ),
                )
            })
    }
}

const RTNLGRP_LINK: u32 = 1;

struct LinkMonitorThread;

impl LinkMonitorThread {
    async fn process(
        to_daemon: Sender<NipartEvent>,
        mut from_plugin: Receiver<BaizeLinkMonitorCmd>,
    ) {
        let mut link_rules: HashMap<String, HashSet<NipartLinkMonitorRule>> =
            HashMap::new();

        let (mut conn, mut _handle, mut messages) =
            match rtnetlink::new_connection() {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Failed to start rtnetlink connection {e}");
                    return;
                }
            };

        let addr = netlink_sys::SocketAddr::new(0, 1 << (RTNLGRP_LINK - 1));

        if let Err(e) = conn.socket_mut().socket_mut().bind(&addr) {
            log::error!("Failed to bind RTNLGRP_LINK: {e}");
            return;
        }

        tokio::spawn(conn);

        loop {
            // TODO: Only start netlink monitor after rules is not empty
            //       Stop netlink monitor after rules is empty.
            tokio::select! {
                Some((message, _)) = messages.next() => {
                    Self::process_netlink_message(
                        message,
                        &mut link_rules,
                        &to_daemon).await;
                },
                Some(cmd) = from_plugin.recv() => {
                    match cmd {
                        BaizeLinkMonitorCmd::AddLinkRule(rule) => {
                            link_rules
                                .entry(rule.iface.clone())
                                .and_modify(|r| {r.insert(rule.clone());})
                                .or_insert_with(|| {
                                    let mut rules = HashSet::new();
                                    rules.insert(rule.clone());
                                    rules
                                });
                        }
                        BaizeLinkMonitorCmd::DelLinkRule(rule) => {
                            if let Some(rules) = link_rules.get_mut(&rule.iface) {
                                rules.retain(|r| r != &rule);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn process_netlink_message(
        message: NetlinkMessage<RouteNetlinkMessage>,
        rules: &mut HashMap<String, HashSet<NipartLinkMonitorRule>>,
        to_daemon: &Sender<NipartEvent>,
    ) {
        log::trace!("Got netlink message {message:?}");
        if let Some((iface, kind)) =
            parse_link_state_from_netlink_message(&message)
        {
            if let Some(iface_rules) = rules.get(iface.as_str()) {
                for rule in iface_rules {
                    if rule.kind == kind {
                        if let Err(e) = send_link_notify(to_daemon, &rule).await
                        {
                            log::error!(
                                "BUG: process_netlink_message failed \
                                to notify {e}"
                            );
                        }
                    }
                }
            }
        }
    }
}

// If interface does not exist, return false.
async fn is_link_up(iface: &str) -> Result<bool, NipartError> {
    let mut iface_filter = nispor::NetStateIfaceFilter::minimum();
    iface_filter.iface_name = Some(iface.to_string());
    let mut filter = nispor::NetStateFilter::minimum();
    filter.iface = Some(iface_filter);
    let np_state = nispor::NetState::retrieve_with_filter_async(&filter)
        .await
        .map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to retried nispor net state: {e}"),
            )
        })?;
    if let Some(iface) = np_state.ifaces.get(iface) {
        Ok(iface.state == nispor::IfaceState::Up)
    } else {
        Ok(false)
    }
}

fn parse_link_state_from_netlink_message(
    message: &NetlinkMessage<RouteNetlinkMessage>,
) -> Option<(String, NipartLinkMonitorKind)> {
    if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewLink(
        link_msg,
    )) = &message.payload
    {
        if let (Some(iface_name), Some(state)) = (
            link_msg.attributes.as_slice().iter().find_map(|attr| {
                if let LinkAttribute::IfName(s) = attr {
                    Some(s.to_string())
                } else {
                    None
                }
            }),
            link_msg.attributes.as_slice().iter().find_map(|attr| {
                if let LinkAttribute::OperState(s) = attr {
                    if *s == netlink_packet_route::link::State::Up {
                        Some(NipartLinkMonitorKind::Up)
                    } else {
                        Some(NipartLinkMonitorKind::Down)
                    }
                } else {
                    None
                }
            }),
        ) {
            return Some((iface_name, state));
        }
        None
    } else {
        None
    }
}

async fn send_link_notify(
    to_daemon: &Sender<NipartEvent>,
    rule: &NipartLinkMonitorRule,
) -> Result<(), NipartError> {
    let monitor_event = match rule.kind {
        NipartLinkMonitorKind::Up => {
            NipartMonitorEvent::LinkUp(rule.iface.clone())
        }
        NipartLinkMonitorKind::Down => {
            NipartMonitorEvent::LinkDown(rule.iface.clone())
        }
        kind => {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!("Unknown NipartLinkMonitorKind {kind}"),
            ));
        }
    };
    let mut reply = NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::GotMonitorEvent(Box::new(monitor_event)),
        NipartEventAddress::Unicast(
            crate::NipartPluginBaize::PLUGIN_NAME.to_string(),
        ),
        rule.requester.clone(),
        nipart::DEFAULT_TIMEOUT,
    );
    reply.uuid = rule.uuid;
    to_daemon.send(reply.clone()).await.map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!("Failed to send event {reply}: {e}"),
        )
    })
}
