// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, NetworkState, NipartError, NipartNoDaemon, NipartQueryOption,
};

use super::{commander::NipartCommander, daemon::DAEMON_IS_ONLINE};

const MAX_RETRY_WAIT: u64 = 32;

impl NipartCommander {
    pub(crate) async fn try_set_daemon_online(
        &mut self,
        saved_state: Option<&NetworkState>,
        cur_state: Option<&NetworkState>,
    ) -> Result<(), NipartError> {
        if DAEMON_IS_ONLINE.initialized() {
            return Ok(());
        }
        let saved_state = if let Some(s) = saved_state {
            s.clone()
        } else {
            self.conf_manager.query_state().await?
        };
        let online_cfg = saved_state.wait_online.unwrap_or_default();
        if online_cfg.conditions.is_empty() {
            // Ignore Err because it only fails when already set which is
            // OK for us to move on.
            DAEMON_IS_ONLINE.set(()).ok();
            return Ok(());
        }

        let cur_state = if let Some(c) = cur_state {
            c.clone()
        } else {
            NipartNoDaemon::query_network_state(NipartQueryOption::running())
                .await?
        };

        if online_cfg
            .conditions
            .into_iter()
            .all(|condition| condition.is_met(&cur_state))
        {
            DAEMON_IS_ONLINE.set(()).ok();
        }

        Ok(())
    }

    pub(crate) async fn wait_online(&mut self) -> Result<(), NipartError> {
        let saved_state = self.conf_manager.query_state().await?;
        let timeout_sec = saved_state
            .wait_online
            .clone()
            .unwrap_or_default()
            .timeout_sec;
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_sec.into()),
            self._wait_online(Some(&saved_state)),
        )
        .await
        {
            Err(_) => Err(NipartError::new(
                ErrorKind::Timeout,
                "Timeout on waiting daemon to reach online state".to_string(),
            )),
            Ok(result) => result,
        }
    }

    async fn _wait_online(
        &mut self,
        saved_state: Option<&NetworkState>,
    ) -> Result<(), NipartError> {
        let mut retry_count = 0;
        // We retry with interval 1 seconds for first 5 seconds for quick boot
        // support, afterwards we wait with exponential increasing time with
        // max 32 seconds.
        while !DAEMON_IS_ONLINE.initialized() {
            self.try_set_daemon_online(saved_state, None).await?;
            let retry_wait = if retry_count > 5 {
                2u64.pow(retry_count - 5).clamp(1, MAX_RETRY_WAIT)
            } else {
                1
            };
            tokio::time::sleep(std::time::Duration::from_secs(retry_wait))
                .await;
            retry_count += 1;
        }
        Ok(())
    }
}
