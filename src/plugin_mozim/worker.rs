// SPDX-License-Identifier: Apache-2.0

use std::os::fd::AsRawFd;

use mozim::{DhcpV4Client, DhcpV4Config, DhcpV4Lease};
use nipart::{
    ErrorKind, NipartDhcpConfigV4, NipartDhcpLease, NipartDhcpLeaseV4,
    NipartError, NipartEvent, NipartEventAddress, NipartLinkMonitorKind,
    NipartLinkMonitorRule, NipartMonitorRule, NipartPluginEvent, NipartRole,
    NipartUserEvent, DEFAULT_TIMEOUT,
};
use tokio::{io::unix::AsyncFd, sync::mpsc::Sender, task::JoinHandle};

const MOZIM_NO_BLOCKING_TIMEOUT: u32 = 0;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum MozimWorkerState {
    /// No DHCP client running yet, only validated the configure.
    WaitLink,
    /// DHCP is running with or without lease.
    Running,
    /// DHCP disabled by user request.
    Disabled,
}

impl std::fmt::Display for MozimWorkerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::WaitLink => "wait_link",
                Self::Running => "running",
                Self::Disabled => "disabled",
            }
        )
    }
}

#[derive(Debug)]
pub(crate) struct MozimWorkerV4Thread {}

impl MozimWorkerV4Thread {
    pub(crate) async fn new(
        iface_name: String,
        mut mozim_client: DhcpV4Client,
        to_daemon: Sender<NipartEvent>,
        event_uuid: u128,
    ) -> Self {
        let fd = match AsyncFd::new(mozim_client.as_raw_fd()) {
            Ok(fd) => fd,
            Err(e) => {
                log::error!(
                    "Mozim worker for {iface_name} event {event_uuid}: \
                    AsyncFd::new() failed with {e}"
                );
                return Self {};
            }
        };
        loop {
            match fd.readable().await {
                Ok(mut guard) => guard.clear_ready(),
                Err(e) => {
                    log::error!(
                        "Mozim worker for {iface_name} event {event_uuid}: \
                        AsyncFd::readable() failed with {e}"
                    );
                    return Self {};
                }
            }
            let mut reply_events = Vec::new();
            let events = match mozim_client.poll(MOZIM_NO_BLOCKING_TIMEOUT) {
                Ok(e) => e,
                Err(e) => {
                    log::error!(
                        "Mozim worker for {iface_name} event {event_uuid}: \
                    mozim_client.poll() failed with {e}"
                    );
                    return Self {};
                }
            };
            let mut has_lease = false;
            for event in events {
                match mozim_client.process(event) {
                    Ok(Some(lease)) => {
                        reply_events.push(gen_dhcp_lease_event(
                            lease,
                            iface_name.as_str(),
                        ));
                        has_lease = true;
                    }
                    Ok(None) => (),
                    Err(e) => {
                        log::error!(
                        "Mozim worker for {iface_name} event {event_uuid}: \
                        mozim_client.process() failed with {e}"
                    );
                        return Self {};
                    }
                }
            }
            for event in reply_events {
                log::debug!("Sending {event}");
                log::trace!("Sending {event:?}");
                if let Err(e) = to_daemon.send(event.clone()).await {
                    log::error!(
                        "Mozim worker for {iface_name} event {event_uuid}: \
                        Failed to send {event}: {e}"
                    );
                    return Self {};
                }
            }
            if has_lease {
                if let Err(e) = register_monitor_on_link_down(
                    &to_daemon,
                    iface_name.as_str(),
                    event_uuid,
                )
                .await
                {
                    log::error!(
                        "Failed to register link down monitor rule for \
                        interface {iface_name}: {e}"
                    );
                    return Self {};
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct MozimWorkerV4 {
    pub(crate) state: MozimWorkerState,
    pub(crate) config: NipartDhcpConfigV4,
    pub(crate) thread_handler: Option<JoinHandle<MozimWorkerV4Thread>>,
    pub(crate) event_uuid: u128,
    pub(crate) to_daemon: Sender<NipartEvent>,
}

impl Drop for MozimWorkerV4 {
    fn drop(&mut self) {
        if let Some(handler) = &self.thread_handler {
            handler.abort();
        }
    }
}

impl MozimWorkerV4 {
    pub(crate) async fn new(
        conf: &NipartDhcpConfigV4,
        event_uuid: u128,
        to_daemon: Sender<NipartEvent>,
    ) -> Result<Self, NipartError> {
        if conf.enabled {
            register_monitor_on_link_up(
                &to_daemon,
                conf.iface.as_str(),
                event_uuid,
            )
            .await?;
            Ok(Self {
                state: MozimWorkerState::WaitLink,
                config: conf.clone(),
                thread_handler: None,
                event_uuid,
                to_daemon,
            })
        } else {
            Ok(Self {
                state: MozimWorkerState::Disabled,
                config: conf.clone(),
                thread_handler: None,
                event_uuid,
                to_daemon,
            })
        }
    }

    pub(crate) fn get_config(&self) -> NipartDhcpConfigV4 {
        self.config.clone()
    }

    /// Please only invoke this function after link up.
    pub(crate) fn start(&mut self) -> Result<(), NipartError> {
        if let Some(handler) = &self.thread_handler {
            log::debug!("Stopping existing DHCP thread before invoke new");
            handler.abort();
        }
        let mozim_config = gen_mozim_config(&self.config);
        // TODO: Load existing lease
        let cli = DhcpV4Client::init(mozim_config, None).map_err(|e| {
            NipartError::new(
                ErrorKind::InvalidArgument,
                format!("Failed to start DHCP {}", e),
            )
        })?;
        self.state = MozimWorkerState::Running;
        let to_daemon = self.to_daemon.clone();
        let event_uuid = self.event_uuid;
        let iface_name = self.config.iface.clone();
        self.thread_handler = Some(tokio::task::spawn(async move {
            MozimWorkerV4Thread::new(iface_name, cli, to_daemon, event_uuid)
                .await
        }));

        Ok(())
    }

    pub(crate) async fn stop(&mut self) {
        if let Some(handler) = &self.thread_handler {
            handler.abort();
            log::debug!(
                "DHCP for interface {} has stopped",
                self.config.iface.as_str()
            );
            if self.config.enabled {
                if let Err(e) = register_monitor_on_link_up(
                    &self.to_daemon,
                    self.config.iface.as_str(),
                    self.event_uuid,
                )
                .await
                {
                    log::error!(
                        "BUG: MozimWorkerV4::stop(): \
                    register_monitor_on_link_up got failure {e}"
                    );
                }
            }
        } else {
            log::debug!(
                "No DHCP thread for interface {} require stop",
                self.config.iface.as_str()
            );
        }
        self.state = MozimWorkerState::Disabled;
        self.thread_handler = None;
    }
}

fn get_prefix_len(ip: &std::net::Ipv4Addr) -> u8 {
    u32::from_be_bytes(ip.octets()).count_ones() as u8
}

fn gen_dhcp_lease_event(
    mozim_lease: DhcpV4Lease,
    iface_name: &str,
) -> NipartEvent {
    let lease = mozim_lease_to_nipart(mozim_lease, iface_name);
    NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::GotDhcpLease(Box::new(lease)),
        NipartEventAddress::Dhcp,
        NipartEventAddress::Commander,
        DEFAULT_TIMEOUT,
    )
}

fn mozim_lease_to_nipart(
    mozim_lease: DhcpV4Lease,
    iface_name: &str,
) -> NipartDhcpLease {
    NipartDhcpLease::V4(NipartDhcpLeaseV4::new(
        iface_name.to_string(),
        mozim_lease.yiaddr,
        get_prefix_len(&mozim_lease.subnet_mask),
        mozim_lease.siaddr,
        mozim_lease.lease_time,
    ))
}

fn gen_mozim_config(conf: &NipartDhcpConfigV4) -> DhcpV4Config {
    let mut mozim_config = DhcpV4Config::new(conf.iface.as_str());
    if let Some(client_id) = conf.client_id.as_ref() {
        mozim_config.set_host_name(client_id);
        mozim_config.use_host_name_as_client_id();
    }
    mozim_config.set_timeout(conf.timeout);
    mozim_config
}

async fn register_monitor_on_link_down(
    to_daemon: &Sender<NipartEvent>,
    iface: &str,
    event_uuid: u128,
) -> Result<(), NipartError> {
    let mut reply = NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::RegisterMonitorRule(Box::new(
            NipartMonitorRule::Link(NipartLinkMonitorRule::new(
                NipartLinkMonitorKind::Down,
                NipartEventAddress::Dhcp,
                event_uuid,
                iface.to_string(),
            )),
        )),
        NipartEventAddress::Dhcp,
        NipartEventAddress::Group(NipartRole::Monitor),
        nipart::DEFAULT_TIMEOUT,
    );
    reply.uuid = event_uuid;
    to_daemon.send(reply).await?;
    Ok(())
}

async fn register_monitor_on_link_up(
    to_daemon: &Sender<NipartEvent>,
    iface: &str,
    event_uuid: u128,
) -> Result<(), NipartError> {
    let mut reply = NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::RegisterMonitorRule(Box::new(
            NipartMonitorRule::Link(NipartLinkMonitorRule::new(
                NipartLinkMonitorKind::Up,
                NipartEventAddress::Dhcp,
                event_uuid,
                iface.to_string(),
            )),
        )),
        NipartEventAddress::Dhcp,
        NipartEventAddress::Group(NipartRole::Monitor),
        nipart::DEFAULT_TIMEOUT,
    );
    reply.uuid = event_uuid;
    log::debug!("Registering link up monitor {reply}");
    to_daemon.send(reply).await?;
    Ok(())
}
