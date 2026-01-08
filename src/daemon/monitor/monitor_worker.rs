// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, HashSet},
    io::Read,
};

use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot::Sender,
};
use futures_util::{SinkExt, stream::StreamExt};
use nipart::{ErrorKind, InterfaceType, NipartError};
use rtnetlink::{
    MulticastGroup, new_multicast_connection,
    packet_core::{NetlinkMessage, NetlinkPayload},
    packet_route::{
        RouteNetlinkMessage,
        link::{
            InfoKind, LinkAttribute, LinkInfo, LinkLayerType, LinkMessage,
            State,
        },
    },
    sys::SocketAddr,
};

use super::super::{
    daemon::NipartManagerCmd,
    event::{NipartLinkEvent, NipartLinkEventType},
    task::TaskWorker,
};

// When the same event happens, how long should consider previous event expired
// and OK to emit the same event again.
const EVENT_EXPIRE_TIME_SEC: u64 = 30;

#[derive(Debug, Clone)]
pub(crate) enum NipartMonitorCmd {
    /// Set the sender for monitor to contact commander. Must be invoked
    /// right after NipartMonitorWorker started.
    SetCommanderSender(UnboundedSender<NipartManagerCmd>),
    /// Start monitoring on specified interface type
    AddIfaceType(InterfaceType),
    /// Stop monitoring on specified interface type
    DelIfaceType(InterfaceType),
    /// Start monitoring on specified interface
    AddIface(String),
    /// Stop monitoring on specified interface
    DelIface(String),
    /// Stop the monitoring but preserving the internal monitoring list
    Pause,
    /// Resume the monitoring, emit current status of monitoring
    /// interface list.
    Resume,
}

impl std::fmt::Display for NipartMonitorCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetCommanderSender(_) => {
                write!(f, "set-commander-sender")
            }
            Self::AddIface(iface) => {
                write!(f, "start-iface-monitor:{iface}")
            }
            Self::DelIface(iface) => {
                write!(f, "stop-iface-monitor:{iface}")
            }
            Self::AddIfaceType(iface_type) => {
                write!(f, "start-iface-type-monitor:{iface_type}")
            }
            Self::DelIfaceType(iface_type) => {
                write!(f, "stop-iface-type-monitor:{iface_type}")
            }
            Self::Pause => {
                write!(f, "pause-monitor")
            }
            Self::Resume => {
                write!(f, "resume-monitor")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NipartMonitorReply {
    None,
}

type FromManager = (NipartMonitorCmd, Sender<Result<NipartMonitorReply, NipartError>>);

#[derive(Debug)]
pub(crate) struct NipartMonitorWorker {
    receiver: UnboundedReceiver<FromManager>,
    netlink_handle: Option<rtnetlink::Handle>,
    netlink_msg_receiver: Option<
        UnboundedReceiver<(NetlinkMessage<RouteNetlinkMessage>, SocketAddr)>,
    >,
    iface_monitor_list: HashSet<String>,
    iface_type_monitor_list: HashSet<InterfaceType>,
    msg_to_commander: Option<UnboundedSender<NipartManagerCmd>>,
    manual_paused: bool,
    emited: HashMap<String, NipartLinkEvent>,
}

impl TaskWorker for NipartMonitorWorker {
    type Cmd = NipartMonitorCmd;
    type Reply = NipartMonitorReply;

    async fn new(
        receiver: UnboundedReceiver<FromManager>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            receiver,
            iface_monitor_list: HashSet::new(),
            iface_type_monitor_list: HashSet::new(),
            netlink_handle: None,
            netlink_msg_receiver: None,
            manual_paused: false,
            msg_to_commander: None,
            emited: HashMap::new(),
        })
    }

    fn receiver(&mut self) -> &mut UnboundedReceiver<FromManager> {
        &mut self.receiver
    }

    async fn process_cmd(
        &mut self,
        cmd: NipartMonitorCmd,
    ) -> Result<NipartMonitorReply, NipartError> {
        log::debug!("Processing monitor command: {cmd}");
        match cmd {
            NipartMonitorCmd::SetCommanderSender(sender) => {
                self.msg_to_commander = Some(sender);
            }
            NipartMonitorCmd::AddIface(iface) => {
                self.iface_monitor_list.insert(iface);
                if self.netlink_msg_receiver.is_none() && !self.manual_paused {
                    self.resume().await?;
                }
            }
            NipartMonitorCmd::DelIface(iface) => {
                self.iface_monitor_list.remove(&iface);
                if self.iface_monitor_list.is_empty() {
                    self.pause();
                }
            }
            NipartMonitorCmd::AddIfaceType(v) => {
                self.iface_type_monitor_list.insert(v);
                if self.netlink_msg_receiver.is_none() && !self.manual_paused {
                    self.resume().await?;
                }
            }
            NipartMonitorCmd::DelIfaceType(v) => {
                self.iface_type_monitor_list.remove(&v);
                if self.iface_type_monitor_list.is_empty() {
                    self.pause();
                }
            }
            NipartMonitorCmd::Pause => {
                self.manual_paused = true;
                self.pause();
            }
            NipartMonitorCmd::Resume => {
                self.manual_paused = false;
                if !self.iface_monitor_list.is_empty()
                    || !self.iface_type_monitor_list.is_empty()
                {
                    self.resume().await?;
                }
            }
        }
        Ok(NipartMonitorReply::None)
    }

    async fn run(&mut self) {
        loop {
            if let Some(mut netlink_msg_receiver) =
                self.netlink_msg_receiver.take()
            {
                tokio::select! {
                    cmd_result = self.recv_cmd() => {
                        if let Some((cmd, sender)) = cmd_result {
                            let cmd_str = cmd.to_string();
                            let result = self.process_cmd(cmd).await;
                            if sender.send(result).is_err() {
                                log::error!(
                                    "Failed to send reply for command {cmd_str}"
                                );
                            }
                        } else {
                            break;
                        }
                    }
                    result = netlink_msg_receiver.next() => {
                        if let Some((nl_msg, _)) = result {
                            if let Err(e) = self.process_rtnl_message(
                                nl_msg,
                            ).await {
                                log::error!("{e}");
                            }
                        }
                    }
                }
                if !self.manual_paused {
                    self.netlink_msg_receiver = Some(netlink_msg_receiver);
                }
            } else if let Some((cmd, sender)) = self.recv_cmd().await {
                let cmd_str = cmd.to_string();
                let result = self.process_cmd(cmd).await;
                if sender.send(result).is_err() {
                    log::error!("Failed to send reply for command {cmd_str}");
                }
            } else {
                break;
            }
        }
    }
}

impl NipartMonitorWorker {
    fn pause(&mut self) {
        self.netlink_handle = None;
        self.netlink_msg_receiver = None;
    }

    async fn notify(&mut self, event: NipartLinkEvent) -> Result<(), NipartError> {
        log::trace!("NipartMonitorWorker sending out {event:?}");
        if let Some(sender) = self.msg_to_commander.as_mut() {
            let cmd = NipartManagerCmd::LinkEvent(Box::new(event.clone()));
            sender.send(cmd).await.map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "NipartMonitorWorker: Failed to send to commander: {e}"
                    ),
                )
            })?;
            self.emited.insert(event.iface_name.to_string(), event);
            Ok(())
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Got NipartMonitorWorker without msg_to_commander: {self:?}"
                ),
            ))
        }
    }

    async fn resume(&mut self) -> Result<(), NipartError> {
        let (conn, handle, msg) =
            new_multicast_connection(&[MulticastGroup::Link]).map_err(|e| {
                NipartError::new(
                    ErrorKind::InvalidArgument,
                    format!(
                        "Failed to create netlink multicast socket for \
                         interface monitor: {e}"
                    ),
                )
            })?;
        tokio::spawn(conn);

        let mut link_handle = handle.link().get().execute();
        while let Some(Ok(link_msg)) = link_handle.next().await {
            if let Some(event) = parse_link_msg(&link_msg)
                && self.should_emit(&event)
            {
                self.notify(event).await?;
            }
        }

        self.netlink_handle = Some(handle);
        self.netlink_msg_receiver = Some(msg);
        Ok(())
    }

    async fn process_rtnl_message(
        &mut self,
        nl_msg: NetlinkMessage<RouteNetlinkMessage>,
    ) -> Result<(), NipartError> {
        if let Some(event) = parse_route_netlink_msg(nl_msg)
            && self.should_emit(&event)
        {
            self.notify(event).await?;
        }
        Ok(())
    }

    fn is_previous_event_expired(&self, event: &NipartLinkEvent) -> bool {
        if let Some(previous_event) = self.emited.get(event.iface_name.as_str())
            && let Ok(elapsed) = previous_event.time_stamp.elapsed()
        {
            previous_event.event_type != event.event_type
                || elapsed
                    > std::time::Duration::from_secs(EVENT_EXPIRE_TIME_SEC)
        } else {
            true
        }
    }

    fn should_emit(&self, event: &NipartLinkEvent) -> bool {
        self.is_previous_event_expired(event)
            && (self.iface_monitor_list.contains(&event.iface_name)
                || self.iface_type_monitor_list.contains(&event.iface_type))
    }
}

fn parse_link_msg(link_msg: &LinkMessage) -> Option<NipartLinkEvent> {
    let iface_name = link_msg.attributes.iter().find_map(|attr| {
        if let &LinkAttribute::IfName(iface_name) = &attr {
            Some(iface_name.to_string())
        } else {
            None
        }
    })?;

    let mut iface_type = parse_iface_type_from_nl_msg(link_msg);
    // The rtnetlink protocol has no information about wireless, so wireless
    // NIC is treated as InterfaceType::Ethernet in rtnetlink.
    if iface_type == InterfaceType::Ethernet && is_wifi_phy_nic(&iface_name) {
        iface_type = InterfaceType::WifiPhy;
    }

    let event_type = if link_msg
        .attributes
        .iter()
        .any(|attr| matches!(attr, LinkAttribute::OperState(State::Up)))
    {
        NipartLinkEventType::CarrierUp
    } else {
        NipartLinkEventType::CarrierDown
    };

    Some(NipartLinkEvent::new(iface_name, iface_type, event_type))
}

fn parse_route_netlink_msg(
    nl_msg: NetlinkMessage<RouteNetlinkMessage>,
) -> Option<NipartLinkEvent> {
    if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewLink(
        link_msg,
    )) = nl_msg.payload
    {
        parse_link_msg(&link_msg)
    } else {
        None
    }
}

fn parse_iface_type_from_nl_msg(link_msg: &LinkMessage) -> InterfaceType {
    if let Some(link_infos) = link_msg.attributes.iter().find_map(|attr| {
        if let LinkAttribute::LinkInfo(infos) = attr {
            Some(infos)
        } else {
            None
        }
    }) && let Some(info_kind) = link_infos.iter().find_map(|info| {
        if let LinkInfo::Kind(k) = info {
            Some(k)
        } else {
            None
        }
    }) {
        match info_kind {
            InfoKind::Bond => InterfaceType::Bond,
            InfoKind::Veth => InterfaceType::Veth,
            InfoKind::Bridge => InterfaceType::LinuxBridge,
            InfoKind::Vlan => InterfaceType::Vlan,
            InfoKind::Vxlan => InterfaceType::Vxlan,
            InfoKind::Dummy => InterfaceType::Dummy,
            InfoKind::Tun => InterfaceType::Tun,
            InfoKind::Vrf => InterfaceType::Vrf,
            InfoKind::MacVlan => InterfaceType::MacVlan,
            InfoKind::MacVtap => InterfaceType::MacVtap,
            InfoKind::Ipoib => InterfaceType::InfiniBand,
            InfoKind::IpVlan => InterfaceType::IpVlan,
            InfoKind::MacSec => InterfaceType::MacSec,
            InfoKind::Hsr => InterfaceType::Hsr,
            InfoKind::Xfrm => InterfaceType::Xfrm,
            v => InterfaceType::Unknown(v.to_string().to_lowercase()),
        }
    } else {
        match link_msg.header.link_layer_type {
            LinkLayerType::Ether => InterfaceType::Ethernet,
            LinkLayerType::Loopback => InterfaceType::Loopback,
            LinkLayerType::Infiniband => InterfaceType::InfiniBand,
            v => InterfaceType::Unknown(v.to_string().to_lowercase()),
        }
    }
}

/// Systemd udev is using `/sys/class/net/{iface_name}/uevent` content
/// `DEVTYPE=wlan` to determine whether wireless or not.
/// And linux kernel code `SET_NETDEV_DEVTYPE(dev, &wiphy_type)` also confirmed
/// so.
fn is_wifi_phy_nic(iface_name: &str) -> bool {
    let mut content = String::new();

    if let Ok(mut fd) =
        std::fs::File::open(format!("/sys/class/net/{iface_name}/uevent"))
        && fd.read_to_string(&mut content).is_ok()
    {
        content.contains("DEVTYPE=wlan")
    } else {
        false
    }
}
