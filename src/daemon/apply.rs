// SPDX-License-Identifier: Apache-2.0

use nipart::{
    Interface, MergedNetworkState, NetworkState, NipartApplyOption,
    NipartError, NipartInterface, NipartIpcConnection, NipartNoDaemon,
};

use super::commander::NipartCommander;
use crate::{log_debug, log_error, log_info, log_trace, log_warn};

const RETRY_COUNT: usize = 10;
const RETRY_INTERVAL_MS: u64 = 500;

impl NipartCommander {
    pub(crate) async fn apply_network_state(
        &mut self,
        mut conn: Option<&mut NipartIpcConnection>,
        mut desired_state: NetworkState,
        opt: NipartApplyOption,
    ) -> Result<NetworkState, NipartError> {
        if desired_state.is_empty() {
            log_info(
                conn.as_deref_mut(),
                "Desired state is empty, no action required".to_string(),
            )
            .await;
        }
        log_trace(
            conn.as_deref_mut(),
            format!("Apply {desired_state} with option {opt}"),
        )
        .await;

        desired_state.ifaces.unify_veth_and_ethernet();

        let mut state_to_save = self.conf_manager.query_state().await?;
        let mut state_to_apply = state_to_save.clone();
        // TODO(Gris): There are many logs shows in this `merge()` process
        // which is not redirected to requested user. We should
        // find a way to redirect these logs.
        state_to_apply.merge(&desired_state)?;
        remove_undesired_ifaces(&mut state_to_apply, &desired_state);

        log_info(
            conn.as_deref_mut(),
            format!(
                "Merged desired with previous saved state, state to apply \
                 {state_to_apply}"
            ),
        )
        .await;
        let mut pre_apply_current_state = self
            .query_network_state(conn.as_deref_mut(), Default::default())
            .await?;

        pre_apply_current_state.ifaces.unify_veth_and_ethernet();

        log_debug(
            conn.as_deref_mut(),
            format!("Pre-apply current state {pre_apply_current_state}"),
        )
        .await;

        let merged_state = MergedNetworkState::new(
            state_to_apply,
            pre_apply_current_state.clone(),
            opt.clone(),
        )?;

        let state_to_apply = merged_state.gen_state_for_apply();

        state_to_save.merge(&state_to_apply)?;

        let revert_state =
            state_to_apply.generate_revert(&pre_apply_current_state)?;

        // TODO(Gris Ge): discard auto IPs

        // Suppress the monitor during applying
        self.monitor_manager.pause().await?;
        if let Err(e) = self
            .apply_merged_state(conn.as_deref_mut(), &merged_state)
            .await
        {
            log_warn(
                conn.as_deref_mut(),
                format!("Failed to apply desired state: {e}"),
            )
            .await;
            log_warn(
                conn.as_deref_mut(),
                format!("Failed to apply merged state: {merged_state}"),
            )
            .await;
            log_warn(
                conn.as_deref_mut(),
                "Rollback to state before apply".to_string(),
            )
            .await;
            log_trace(
                conn.as_deref_mut(),
                format!("Rollback to state before apply {revert_state}"),
            )
            .await;
            if let Err(e) =
                self.rollback(conn.as_deref_mut(), revert_state).await
            {
                log_error(
                    conn.as_deref_mut(),
                    format!("Failed to rollback: {e}"),
                )
                .await;
            }
            return Err(e);
        }

        if let Err(e) =
            self.conf_manager.save_state(state_to_save.clone()).await
        {
            log_warn(
                conn.as_deref_mut(),
                format!(
                    "BUG: Failed to persistent desired state {state_to_save}: \
                     {e}"
                ),
            )
            .await;
        }

        let saved_state = self.conf_manager.query_state().await?;

        self.monitor_manager
            .setup_monitor(&merged_state, &saved_state)
            .await?;

        self.monitor_manager.resume().await?;

        let mut diff_state = match merged_state
            .gen_state_for_apply()
            .gen_diff(&pre_apply_current_state)
        {
            Ok(s) => s,
            Err(e) => {
                log_warn(
                    conn,
                    format!("Returning full state instead of diff state: {e}"),
                )
                .await;
                merged_state.gen_state_for_apply()
            }
        };
        diff_state.hide_secrets();

        self.try_set_daemon_online(Some(&saved_state), None).await?;

        Ok(diff_state)
    }

    async fn rollback(
        &mut self,
        mut conn: Option<&mut NipartIpcConnection>,
        revert_state: NetworkState,
    ) -> Result<(), NipartError> {
        let mut opt = NipartApplyOption::default();
        opt.no_verify = true;

        let current_state = self
            .query_network_state(conn.as_deref_mut(), Default::default())
            .await?;
        let merged_state =
            MergedNetworkState::new(revert_state, current_state, opt.clone())?;

        let apply_state = merged_state.gen_state_for_apply();

        NipartNoDaemon::apply_merged_state(&merged_state).await?;
        self.plugin_manager
            .apply_network_state(&apply_state, &opt)
            .await?;

        self.dhcpv4_manager
            .apply_dhcp_config(conn, &merged_state)
            .await?;

        Ok(())
    }

    async fn verify(
        &mut self,
        mut conn: Option<&mut NipartIpcConnection>,
        merged_state: &MergedNetworkState,
    ) -> Result<(), NipartError> {
        let mut post_apply_current_state = self
            .query_network_state(conn.as_deref_mut(), Default::default())
            .await?;
        // The wifi config is not stored into config manager yet. In order to
        // pass the verification, we need to pretend the wifi config is stored
        // in config manager.
        for iface in merged_state.ifaces.user_ifaces.values().filter_map(
            |merged_iface| {
                if let Some(Interface::WifiCfg(iface)) =
                    merged_iface.desired.as_ref()
                {
                    Some(iface)
                } else {
                    None
                }
            },
        ) {
            post_apply_current_state
                .ifaces
                .push(Interface::WifiCfg(Box::new(*iface.clone())));
        }

        // Up/down trigger is not stored into config manager yet. In order to
        // pass the verification, we need to pretend the up/down trigger is
        // stored.
        for merged_iface in merged_state.ifaces.iter() {
            if let Some(post_apply_iface) =
                post_apply_current_state.ifaces.get_mut(
                    merged_iface.merged.name(),
                    Some(merged_iface.merged.iface_type()),
                )
            {
                post_apply_iface.base_iface_mut().trigger = merged_iface
                    .for_apply
                    .as_ref()
                    .and_then(|i| i.base_iface().trigger.clone());
            }
        }

        log_trace(
            conn,
            format!("Post apply network state: {post_apply_current_state}"),
        )
        .await;
        merged_state.verify(&post_apply_current_state)?;
        self.try_set_daemon_online(None, Some(&post_apply_current_state))
            .await?;
        Ok(())
    }

    // Apply state to plugin/dhcp/kernel and verify, but don't do these tasks:
    //  * Checkpoint rollback
    //  * Config save
    //  * Setup monitor session
    pub(crate) async fn apply_merged_state(
        &mut self,
        mut conn: Option<&mut NipartIpcConnection>,
        merged_state: &MergedNetworkState,
    ) -> Result<(), NipartError> {
        let apply_state = merged_state.gen_state_for_apply();

        log_trace(conn.as_deref_mut(), format!("apply_state {apply_state}"))
            .await;

        let mut merged_state_for_no_daemon = merged_state.clone();
        // Remove interfaces for conditional activating
        merged_state_for_no_daemon.remove_conditional_activation();

        NipartNoDaemon::apply_merged_state(&merged_state_for_no_daemon).await?;
        self.plugin_manager
            .apply_network_state(&apply_state, &merged_state.option)
            .await?;

        self.dhcpv4_manager
            .apply_dhcp_config(conn.as_deref_mut(), merged_state)
            .await?;

        let mut result: Result<(), NipartError> = Ok(());
        if !merged_state.option.no_verify {
            for cur_retry_count in 1..(RETRY_COUNT + 1) {
                result = self
                    .verify(conn.as_deref_mut(), &merged_state_for_no_daemon)
                    .await;
                if let Err(e) = &result {
                    log_info(
                        conn.as_deref_mut(),
                        format!(
                            "Retrying({cur_retry_count}/{RETRY_COUNT}) on \
                             verification error: {e}"
                        ),
                    )
                    .await;
                    tokio::time::sleep(std::time::Duration::from_millis(
                        RETRY_INTERVAL_MS,
                    ))
                    .await;
                } else {
                    break;
                }
            }
        }
        result
    }
}

fn remove_undesired_ifaces(
    merged_desired_state: &mut NetworkState,
    desired_state: &NetworkState,
) {
    merged_desired_state
        .ifaces
        .kernel_ifaces
        .retain(|iface_name, _| {
            desired_state
                .ifaces
                .kernel_ifaces
                .contains_key(&iface_name.to_string())
        });
    merged_desired_state.ifaces.user_ifaces.retain(|key, _| {
        desired_state
            .ifaces
            .user_ifaces
            .contains_key(&(key.clone()))
    });
}
