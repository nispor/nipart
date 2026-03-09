// SPDX-License-Identifier: Apache-2.0

use futures_channel::{mpsc::UnboundedReceiver, oneshot::Sender};
use nipart::{
    BaseInterface, ErrorKind, Interface, InterfaceIpv4, InterfaceIpv6,
    InterfaceState, InterfaceTrigger, InterfaceType, MergedNetworkState,
    NetworkState, NipartApplyOption, NipartError, NipartInterface,
    NipartNoDaemon, NipartQueryOption,
};

use super::super::{
    commander::NipartCommander, link_event::NipartLinkEvent, task::TaskWorker,
};

#[derive(Debug, Clone)]
pub(crate) enum NipartEventCmd {
    SetCommander(Box<NipartCommander>),
    HandleEvent(Box<NipartLinkEvent>),
}

impl std::fmt::Display for NipartEventCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetCommander(_) => {
                write!(f, "set-commander")
            }
            Self::HandleEvent(event) => {
                write!(f, "handle-event:{event}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NipartEventReply {
    None,
}

type FromManager = (
    NipartEventCmd,
    Sender<Result<NipartEventReply, NipartError>>,
);

#[derive(Debug)]
pub(crate) struct NipartEventWorker {
    receiver: UnboundedReceiver<FromManager>,
    commander: Option<NipartCommander>,
}

impl TaskWorker for NipartEventWorker {
    type Cmd = NipartEventCmd;
    type Reply = NipartEventReply;

    async fn new(
        receiver: UnboundedReceiver<FromManager>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            receiver,
            commander: None,
        })
    }

    fn receiver(&mut self) -> &mut UnboundedReceiver<FromManager> {
        &mut self.receiver
    }

    async fn process_cmd(
        &mut self,
        cmd: NipartEventCmd,
    ) -> Result<NipartEventReply, NipartError> {
        log::debug!("Processing event command: {cmd}");
        match cmd {
            NipartEventCmd::SetCommander(commander) => {
                self.commander = Some(*commander);
            }
            NipartEventCmd::HandleEvent(event) => {
                if let Err(e) = self.handle_event(*event).await {
                    log::error!("{e}");
                }
            }
        }
        Ok(NipartEventReply::None)
    }
}

impl NipartEventWorker {
    async fn handle_event(
        &mut self,
        mut event: NipartLinkEvent,
    ) -> Result<(), NipartError> {
        let Some(commander) = self.commander.as_mut() else {
            return Err(NipartError::new(
                ErrorKind::Bug,
                "NipartEventWorker::handle_event() invoked without commander \
                 set"
                .to_string(),
            ));
        };
        log::trace!("Handle link event {event}");
        let saved_state = commander.conf_manager.query_state().await?;
        let cur_state =
            NipartNoDaemon::query_network_state(NipartQueryOption::running())
                .await?;

        let cur_iface = cur_state
            .ifaces
            .get(&event.iface_name, Some(&event.iface_type));
        if let Some(cur_iface) = cur_iface {
            log::trace!("Current interface state: {cur_iface}");

            if event.ssid.is_none()
                && event.iface_type == InterfaceType::WifiPhy
                && let Interface::WifiPhy(cur_wifi_iface) = cur_iface
            {
                event.ssid = cur_wifi_iface.ssid().map(|s| s.to_string());
            }
        }

        let mut desired_state = NetworkState::default();

        // Purge IP if WIFI PHY interface is down or removed
        if !event.is_up && event.iface_type == InterfaceType::WifiPhy {
            let mut desired_iface = BaseInterface::new(
                event.iface_name.to_string(),
                event.iface_type.clone(),
            );
            desired_iface.state = if cur_iface.is_some() {
                InterfaceState::Up
            } else {
                // WIFI PHY interface removed.
                InterfaceState::Absent
            };
            // Purge IP
            desired_iface.ipv4 = Some(InterfaceIpv4::new_disabled());
            desired_iface.ipv6 = Some(InterfaceIpv6::new_disabled());
            desired_state.ifaces.push(desired_iface.into());
        }

        for saved_iface in saved_state.ifaces.iter() {
            if !event.is_up
                && saved_iface.iface_type() == &InterfaceType::WifiPhy
            {
                // Already processed above.
                continue;
            }

            // When new WIFI PHY found, we should setup `bind-to-any` WIFI to
            // it.
            if !event.is_up
                && !event.is_delete
                && event.iface_type == InterfaceType::WifiPhy
                && let Interface::WifiCfg(saved_wifi_cfg) = saved_iface
                && (saved_wifi_cfg.parent().is_none()
                    || saved_wifi_cfg.parent()
                        == Some(event.iface_name.as_str()))
            {
                let mut desired_iface = saved_iface.clone();
                // WifiCfg bind to any SSID should changed to event
                // interface only, so other interface is not impacted
                if let Interface::WifiCfg(iface) = &mut desired_iface
                    && let Some(wifi_cfg) = iface.wifi.as_mut()
                {
                    wifi_cfg.base_iface = Some(event.iface_name.to_string());
                }
                desired_state.ifaces.push(desired_iface);
                continue;
            }

            // WIFI PHY connected to SSID, should use setting of WIFI CFG.
            if event.is_up
                && let Some(Interface::WifiPhy(cur_wifi_iface)) = cur_iface
                && cur_wifi_iface.ssid().is_some()
                && let Interface::WifiCfg(saved_wifi_iface) = saved_iface
                && cur_wifi_iface.ssid() == saved_wifi_iface.ssid()
            {
                desired_state.ifaces.push(wifi_cfg_to_wifi_phy(
                    event.iface_name.as_str(),
                    saved_iface,
                ));
                continue;
            }

            let trigger =
                saved_iface.base_iface().trigger.clone().unwrap_or_default();

            match trigger.process(
                event.iface_name.as_str(),
                &event.iface_type,
                &cur_state.ifaces,
            ) {
                None => {
                    continue;
                }
                Some(false) => {
                    let desired_iface =
                        gen_desired_iface_down(&trigger, saved_iface);
                    desired_state.ifaces.push(desired_iface);
                }
                Some(true) => {
                    let desired_iface = gen_desired_iface_up(
                        saved_iface,
                        &saved_state,
                        &mut desired_state,
                    );
                    desired_state.ifaces.push(desired_iface);
                }
            }
        }

        if !desired_state.is_empty() {
            log::trace!("Applying desired state {desired_state}");
            let merged_state = MergedNetworkState::new(
                desired_state,
                cur_state,
                NipartApplyOption::new().no_verify(),
            )?;
            commander.apply_merged_state(None, &merged_state).await?;
        } else {
            log::trace!("No change required for event {event}");
        }

        Ok(())
    }
}

fn gen_desired_iface_up(
    saved_iface: &Interface,
    saved_state: &NetworkState,
    desired_state: &mut NetworkState,
) -> Interface {
    let mut desired_iface = saved_iface.clone();
    desired_iface.base_iface_mut().state = InterfaceState::Up;
    desired_iface.base_iface_mut().trigger = None;

    // Include routes to this interface also
    if !desired_iface.is_userspace()
        && let Some(config_rts) = saved_state.routes.config.as_ref()
    {
        for rt in config_rts {
            if rt.next_hop_iface.as_deref() == Some(saved_iface.name()) {
                desired_state
                    .routes
                    .config
                    .get_or_insert(Vec::new())
                    .push(rt.clone());
            }
        }
    }

    desired_iface
}

fn gen_desired_iface_down(
    trigger: &InterfaceTrigger,
    saved_iface: &Interface,
) -> Interface {
    let mut desired_iface = saved_iface.clone();
    if trigger != &InterfaceTrigger::Carrier
        && saved_iface.iface_type() != &InterfaceType::WifiCfg
    {
        desired_iface.base_iface_mut().state = if saved_iface.is_virtual() {
            InterfaceState::Absent
        } else {
            InterfaceState::Down
        };
    }
    desired_iface.base_iface_mut().trigger = None;
    desired_iface.base_iface_mut().ipv4 = Some(InterfaceIpv4::new_disabled());
    desired_iface.base_iface_mut().ipv6 = Some(InterfaceIpv6::new_disabled());

    desired_iface
}

fn wifi_cfg_to_wifi_phy(
    iface_name: &str,
    saved_iface: &Interface,
) -> Interface {
    let mut desired = saved_iface.base_iface().clone();
    desired.name = iface_name.to_string();
    desired.iface_type = InterfaceType::WifiPhy;

    desired.into()
}
