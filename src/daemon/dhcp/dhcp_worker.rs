// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
    oneshot::Sender,
};
use futures_util::StreamExt;
use mozim::{DhcpV4Client, DhcpV4Config, DhcpV4Lease, DhcpV4State};
use nipart::{
    BaseInterface, DhcpState, ErrorKind, Interface, InterfaceIpAddr,
    InterfaceIpv4, NetworkState, NipartError, NipartNoDaemon, NipartstateApplyOption,
    RouteEntry, Routes,
};

use crate::TaskWorker;

const DEFAULT_ROUTE_TABLE_ID: u32 = 254;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NipartDhcpCmd {
    StartIfaceDhcp(Box<BaseInterface>),
    StopIfaceDhcp(String),
    Query,
}

impl std::fmt::Display for NipartDhcpCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartIfaceDhcp(base_iface) => {
                write!(f, "start-iface-dhcp:{}", base_iface.name)
            }
            Self::StopIfaceDhcp(iface) => {
                write!(f, "stop-iface-dhcp:{iface}")
            }
            Self::Query => {
                write!(f, "query-dhcp")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NipartDhcpReply {
    None,
    QueryReply(HashMap<String, DhcpState>),
}

type FromManager = (NipartDhcpCmd, Sender<Result<NipartDhcpReply, NipartError>>);

#[derive(Debug)]
pub(crate) struct NipartDhcpV4Worker {
    threads: HashMap<String, NipartDhcpV4Thread>,
    receiver: UnboundedReceiver<FromManager>,
}

impl TaskWorker for NipartDhcpV4Worker {
    type Cmd = NipartDhcpCmd;
    type Reply = NipartDhcpReply;

    async fn new(
        receiver: UnboundedReceiver<(
            Self::Cmd,
            Sender<Result<Self::Reply, NipartError>>,
        )>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            threads: HashMap::new(),
            receiver,
        })
    }

    fn receiver(&mut self) -> &mut UnboundedReceiver<FromManager> {
        &mut self.receiver
    }

    async fn process_cmd(
        &mut self,
        cmd: NipartDhcpCmd,
    ) -> Result<NipartDhcpReply, NipartError> {
        match cmd {
            NipartDhcpCmd::StartIfaceDhcp(base_iface) => {
                let iface_name = base_iface.name.clone();
                let thread = NipartDhcpV4Thread::new(*base_iface).await?;
                self.threads.insert(iface_name, thread);
                Ok(NipartDhcpReply::None)
            }
            NipartDhcpCmd::StopIfaceDhcp(iface) => {
                self.threads.remove(&iface);
                Ok(NipartDhcpReply::None)
            }
            NipartDhcpCmd::Query => {
                let mut ret = HashMap::new();
                for (iface_name, thread) in self.threads.iter() {
                    ret.insert(iface_name.to_string(), thread.get_state()?);
                }

                Ok(NipartDhcpReply::QueryReply(ret))
            }
        }
    }
}

#[derive(Debug, Default)]
struct NipartDhcpShareData {
    state: DhcpState,
}

#[derive(Debug)]
pub(crate) struct NipartDhcpV4Thread {
    pub(crate) base_iface: BaseInterface,
    // No need to send any data. Dropping this Sender will cause
    // Receiver.recv() got None which trigger DHCP thread quit.
    _quit_notifer: UnboundedSender<()>,
    share_data: Arc<Mutex<NipartDhcpShareData>>,
}

impl NipartDhcpV4Thread {
    pub(crate) async fn new(
        base_iface: BaseInterface,
    ) -> Result<Self, NipartError> {
        let (sender, receiver) = unbounded();
        let ret = Self {
            base_iface: base_iface.clone(),
            _quit_notifer: sender,
            share_data: Arc::new(Mutex::new(NipartDhcpShareData::default())),
        };
        let mac_addr = match base_iface.mac_address.as_deref() {
            Some(m) => m,
            None => {
                return Err(NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Got no MAC address for DHCPv4 on interface {}({})",
                        base_iface.name, base_iface.iface_type
                    ),
                ));
            }
        };
        let iface_index = match base_iface.iface_index {
            Some(m) => m,
            None => {
                return Err(NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Got no interface index for DHCPv4 on interface {}({})",
                        base_iface.name, base_iface.iface_type
                    ),
                ));
            }
        };
        let mut dhcp_config = DhcpV4Config::new(base_iface.name.as_str());
        dhcp_config
            .set_iface_index(iface_index)
            .set_iface_mac(mac_addr)
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to set iface {}/{} MAC {}: {e}",
                        base_iface.name, base_iface.iface_type, mac_addr,
                    ),
                )
            })?
            .use_mac_as_client_id();
        // TODO(Gris Ge): Support loading previous stored lease
        let dhcp_client =
            DhcpV4Client::init(dhcp_config, None).await.map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to start DHCPv4 client on iface {}/{}: {e}",
                        base_iface.name, base_iface.iface_type,
                    ),
                )
            })?;

        let share_data = ret.share_data.clone();
        tokio::spawn(async move {
            if let Err(e) =
                dhcp_thread(dhcp_client, base_iface, receiver, share_data).await
            {
                log::error!("{e}");
            }
        });
        Ok(ret)
    }

    pub(crate) fn get_state(&self) -> Result<DhcpState, NipartError> {
        match self.share_data.lock() {
            Ok(data) => Ok(data.state.clone()),
            Err(e) => Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Failed to lock share data of DHCP thread for interface \
                     {}: {e}",
                    self.base_iface.name
                ),
            )),
        }
    }
}

async fn dhcp_thread(
    mut dhcp_client: DhcpV4Client,
    base_iface: BaseInterface,
    mut quit_indicator: UnboundedReceiver<()>,
    share_data: Arc<Mutex<NipartDhcpShareData>>,
) -> Result<(), NipartError> {
    log::debug!(
        "Waiting link carrier up for interface {}/{} before start DHCP",
        base_iface.name,
        base_iface.iface_type
    );
    NipartNoDaemon::wait_link_carrier_up(base_iface.name.as_str()).await?;
    log::debug!(
        "Interface {}/{} link carrier is up, starting DHCP process",
        base_iface.name,
        base_iface.iface_type
    );
    match share_data.lock() {
        Ok(mut share_data) => {
            share_data.state = DhcpState::Running;
        }
        Err(e) => {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Failed to lock DHCPv4 {}({}) share data: {e}",
                    base_iface.name, base_iface.iface_type,
                ),
            ));
        }
    }
    let result = loop {
        tokio::select! {
            result = dhcp_client.run() => {
                match result {
                    Ok(DhcpV4State::Done(lease)) => {
                        log::info!(
                            "DHCPv4 on {}({}) got lease {}",
                            base_iface.name,
                            base_iface.iface_type,
                            lease.yiaddr,
                        );
                        match share_data.lock() {
                            Ok(mut share_data) => {
                                share_data.state = DhcpState::Done;
                            }
                            Err(e) => {
                                break Err::<(), NipartError>(NipartError::new(
                                    ErrorKind::Bug,
                                    format!("Unhandled DHCPv4 error: {e}"),
                                ));
                            }
                        }
                        if let Err(e) = apply_lease(
                            &base_iface,
                            &lease,
                            share_data.clone()
                        ).await {
                            break Err(e);
                        }
                    }
                    Ok(dhcp_state) => {
                        log::info!(
                            "DHCPv4 on {}({}) reach {} state",
                            base_iface.name,
                            base_iface.iface_type,
                            dhcp_state
                        );
                    }
                    Err(e) => {
                        break Err(NipartError::new(
                            ErrorKind::Bug,
                            format!("Unhandled DHCPv4 error: {e}"),
                        ));
                    }
                }
            }
            _ = quit_indicator.next() => {
                log::info!(
                    "DHCPv4 on {}({}) stopped",
                    base_iface.name,
                    base_iface.iface_type,
                );
                return Ok(());
            }
        }
    };

    if let Err(e) = result {
        match share_data.lock() {
            Ok(mut share_data) => {
                share_data.state = DhcpState::Error(e.to_string());
            }
            Err(e) => {
                return Err(NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to lock DHCPv4 {}({}) share data: {e}",
                        base_iface.name, base_iface.iface_type,
                    ),
                ));
            }
        }
    }
    Ok(())
}

async fn apply_lease(
    base_iface: &BaseInterface,
    lease: &DhcpV4Lease,
    // TODO: Support hostname, systemd-resolved, /etc/resolv.conf
    _share_data: Arc<Mutex<NipartDhcpShareData>>,
) -> Result<(), NipartError> {
    log::debug!(
        "Applying DHCPv4 lease {}/{} to interface {}({})",
        lease.yiaddr,
        lease.prefix_length(),
        base_iface.name,
        base_iface.iface_type
    );

    let mut ip_addr =
        InterfaceIpAddr::new(lease.yiaddr.into(), lease.prefix_length());
    ip_addr.preferred_life_time = Some(format!("{}sec", lease.lease_time_sec));
    ip_addr.valid_life_time = Some(format!("{}sec", lease.lease_time_sec));

    let mut ipv4_conf = InterfaceIpv4::new();
    ipv4_conf.enabled = Some(true);
    ipv4_conf.dhcp = Some(true);
    ipv4_conf.addresses = Some(vec![ip_addr]);

    let mut apply_base_iface = base_iface.clone_name_type_only();

    apply_base_iface.ipv4 = Some(ipv4_conf);
    if let Some(mtu) = lease.mtu {
        apply_base_iface.mtu = Some(mtu.into());
    }

    let iface_state: Interface = apply_base_iface.into();
    let mut net_state = NetworkState::new();
    net_state.ifaces.push(iface_state);

    net_state.routes = gen_routes(lease, base_iface);

    let apply_opt = NipartstateApplyOption::new().no_verify();
    NipartNoDaemon::apply_network_state(net_state, apply_opt).await?;
    Ok(())
}

// TODO:
//  * Handle `auto-routes: false`
//  * Handle `auto-gateways: false`
//  * Handle `classless_routes`
fn gen_routes(lease: &DhcpV4Lease, base_iface: &BaseInterface) -> Routes {
    let mut conf_routes: Vec<RouteEntry> = Vec::new();
    // TODO: Handle multiple addresses of router
    if let Some(gateways) = lease.gateways.as_ref() {
        for (index, gateway) in gateways.iter().enumerate() {
            let mut route = RouteEntry::default();
            route.destination = Some("0.0.0.0/0".to_string());
            route.next_hop_iface = Some(base_iface.name.to_string());
            route.next_hop_addr = Some(gateway.to_string());
            route.table_id = Some(DEFAULT_ROUTE_TABLE_ID);
            // TODO: Be consistent on metric?
            // TODO: Priority ethernet over wifi/VPN/etc ?
            route.metric = base_iface
                .iface_index
                .map(|iface_index| 100i64 * iface_index as i64 + index as i64);
            conf_routes.push(route);
        }
    }

    let mut routes = Routes::default();
    routes.config = Some(conf_routes);
    routes
}
