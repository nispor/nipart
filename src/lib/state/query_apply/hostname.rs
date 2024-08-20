// SPDX-License-Identifier: Apache-2.0

use crate::{ErrorKind, HostNameState, MergedHostNameState, NipartError};

impl HostNameState {
    pub(crate) fn update(&mut self, other: &Self) {
        if other.running.is_some() {
            self.running.clone_from(&other.running);
        }
        if other.config.is_some() {
            self.config.clone_from(&other.config);
        }
    }
}

impl MergedHostNameState {
    pub fn is_changed(&self) -> bool {
        self.desired != self.current
    }

    pub(crate) fn verify(
        &self,
        current: Option<&HostNameState>,
    ) -> Result<(), NipartError> {
        let desired = if let Some(d) = &self.desired {
            d
        } else {
            return Ok(());
        };
        let current = if let Some(c) = current {
            c
        } else {
            return Err(NipartError::new(
                ErrorKind::Bug,
                "MergedHostNameState::verify(): Got current \
                HostNameState set to None"
                    .to_string(),
            ));
        };

        if let Some(running) = desired.running.as_ref() {
            if Some(running) != current.running.as_ref() {
                let e = NipartError::new(
                    ErrorKind::VerificationError,
                    format!(
                        "Verification fail, desire hostname.running: \
                        {}, current: {:?}",
                        running,
                        current.running.as_ref()
                    ),
                );
                log::error!("{}", e);
                return Err(e);
            }
        }
        if let Some(config) = desired.config.as_ref() {
            if Some(config) != current.config.as_ref() {
                let e = NipartError::new(
                    ErrorKind::VerificationError,
                    format!(
                        "Verification fail, desire hostname.config: \
                        {}, current: {:?}",
                        config,
                        current.config.as_ref()
                    ),
                );
                log::error!("{}", e);
                return Err(e);
            }
        }

        Ok(())
    }
}
