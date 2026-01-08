// SPDX-License-Identifier: Apache-2.0

use super::{inter_ifaces::apply_ifaces, route::apply_routes};
use crate::{
    InterfaceType, MergedNetworkState, NetworkState, NipartError, NipartNoDaemon,
    NipartstateApplyOption, NipartstateInterface,
};

const RETRY_COUNT_COMMON: usize = 10;
const RETRY_COUNT_WIFI: usize = 20;
const RETRY_INTERVAL_MS: u64 = 500;

impl NipartNoDaemon {
    pub async fn apply_network_state(
        desired_state: NetworkState,
        option: NipartstateApplyOption,
    ) -> Result<NetworkState, NipartError> {
        let current_state =
            Self::query_network_state(Default::default()).await?;

        log::trace!("Applying {desired_state} with option {option}");
        let merged_state = MergedNetworkState::new(
            desired_state.clone(),
            current_state.clone(),
            option.clone(),
        )?;

        for iface in
            merged_state.ifaces.iter().filter(|i| i.for_apply.is_some())
        {
            if let Some(cur_iface) = iface.current.as_ref() {
                log::trace!("Current interface {cur_iface}");
            }
            if let Some(apply_iface) = iface.for_apply.as_ref() {
                log::trace!("Applying interface changes: {apply_iface}");
            }
        }

        // TODO(Gris Ge): Special sanitize for NoDaemon mode:
        //  * DHCP not supported
        //  * controller and IP setting for `wifi-cfg` interface

        Self::apply_merged_state(&merged_state).await?;
        if option.dhcp_in_no_daemon {
            Self::run_dhcp_once(&merged_state.ifaces).await?;
        }

        let max_retry_count = get_max_retry_count(&merged_state);

        let mut result: Result<(), NipartError> = Ok(());
        if !option.no_verify {
            for cur_retry_count in 1..(max_retry_count + 1) {
                let post_apply_current_state =
                    Self::query_network_state(Default::default()).await?;
                log::trace!(
                    "Post apply network state: {post_apply_current_state}"
                );
                if cur_retry_count == max_retry_count / 2 {
                    log::info!("Apply the desired state again");
                    Self::apply_merged_state(&merged_state).await?;
                    if option.dhcp_in_no_daemon {
                        Self::run_dhcp_once(&merged_state.ifaces).await?;
                    }
                }
                result = merged_state.verify(&post_apply_current_state);
                if let Err(e) = &result {
                    log::info!(
                        "Retrying({cur_retry_count}/{max_retry_count}) on \
                         verification error: {e}"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(
                        RETRY_INTERVAL_MS,
                    ))
                    .await;
                } else {
                    break;
                }
            }
        }
        result?;

        let diff_state = merged_state
            .gen_state_for_apply()
            .gen_diff(&current_state)?;

        Ok(diff_state)
    }

    pub async fn apply_merged_state(
        merged_state: &MergedNetworkState,
    ) -> Result<(), NipartError> {
        apply_ifaces(&merged_state.ifaces).await?;
        apply_routes(&merged_state.routes).await?;
        Ok(())
    }
}

fn get_max_retry_count(merged_state: &MergedNetworkState) -> usize {
    if merged_state
        .ifaces
        .kernel_ifaces
        .values()
        .any(|merged_iface| {
            merged_iface.for_apply.is_some()
                && matches!(
                    merged_iface.merged.iface_type(),
                    InterfaceType::WifiPhy | InterfaceType::WifiCfg
                )
        })
    {
        RETRY_COUNT_WIFI
    } else {
        RETRY_COUNT_COMMON
    }
}
