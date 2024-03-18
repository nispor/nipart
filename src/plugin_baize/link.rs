// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use futures::stream::StreamExt;
use netlink_packet_core::NetlinkMessage;
use netlink_packet_route::RouteNetlinkMessage;
use netlink_sys::AsyncSocket;
use nipart::{
    ErrorKind, NipartError, NipartEvent, NipartEventAddress,
    NipartLinkMonitorRule, NipartMonitorEvent, NipartNativePlugin,
    NipartPluginEvent, NipartUserEvent,
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
        let thread_handler = tokio::task::spawn(async move {
            LinkMonitorThread::process(to_daemon, plugin_to_monitor_rx).await
        });

        Ok(Self {
            thread_handler,
            to_monitor: plugin_to_monitor_tx,
        })
    }

    // TODO: Upon the new rule insertion, we should check current status of this
    // link and generate reply instantly if condition already met.
    pub(crate) async fn add_link_rule(
        &mut self,
        rule: NipartLinkMonitorRule,
    ) -> Result<(), NipartError> {
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
        let mut link_rules: HashMap<String, Vec<NipartLinkMonitorRule>> =
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
                    process_netlink_message(
                        message,
                        &link_rules,
                        &to_daemon).await;
                },
                Some(cmd) = from_plugin.recv() => {
                    match cmd {
                        BaizeLinkMonitorCmd::AddLinkRule(rule) => {
                            link_rules
                                .entry(rule.iface.clone())
                                .and_modify(|r| r.push(rule.clone()))
                                .or_insert(vec![rule.clone()]);
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
}

async fn process_netlink_message(
    message: NetlinkMessage<RouteNetlinkMessage>,
    rules: &HashMap<String, Vec<NipartLinkMonitorRule>>,
    _to_daemon: &Sender<NipartEvent>,
) {
    log::error!("HAHA rules {:?}", rules);
    log::error!("HHA rt message {:?}", message);
}

// If interface does not exist, return false.
pub(crate) async fn is_link_up(iface: &str) -> Result<bool, NipartError> {
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

pub(crate) async fn send_link_up_notify(
    to_daemon: &Sender<NipartEvent>,
    rule: &NipartLinkMonitorRule,
) -> Result<(), NipartError> {
    let mut reply = NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::GotMonitorEvent(Box::new(
            NipartMonitorEvent::LinkUp(rule.iface.clone()),
        )),
        NipartEventAddress::Unicast(
            crate::NipartPluginBaize::PLUGIN_NAME.to_string(),
        ),
        rule.requestee.clone(),
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
