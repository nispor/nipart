// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::{
    ErrorKind, NetworkState, NipartError, NipartIpcConnection,
    NipartIpcListener, NipartPluginClient, NipartPluginCmd, NipartPluginInfo,
    NipartstateApplyOption, NipartstateQueryOption,
};

pub trait NipartPlugin: Send + Sync + Sized + 'static {
    const PLUGIN_NAME: &'static str;

    fn init() -> impl Future<Output = Result<Self, NipartError>> + Send;

    /// Default implementation is `std::process::exit(0)`
    fn quit(_plugin: &Arc<Self>) -> impl Future<Output = ()> + Send {
        async {
            std::process::exit(0);
        }
    }

    fn plugin_info(
        plugin: &Arc<Self>,
    ) -> impl Future<Output = Result<NipartPluginInfo, NipartError>> + Send;

    /// The `&self` will cloned and move to forked thread for each connection.
    fn run() -> impl Future<Output = Result<(), NipartError>> + Send {
        let mut log_builder = env_logger::Builder::new();
        log_builder.filter(Some("nm"), log::LevelFilter::Debug);
        log_builder.filter(Some("nm_plugin"), log::LevelFilter::Debug);
        log_builder.filter(
            Some(&format!("nipart-plugin-{}", Self::PLUGIN_NAME)),
            log::LevelFilter::Debug,
        );
        log_builder.init();

        // TODO(Gris Ge): Do we need to ping daemon to make sure daemon is
        // still alive?
        async {
            let plugin = Arc::new(Self::init().await?);

            let socket_path = format!(
                "{}/{}",
                NipartPluginClient::DEFAULT_SOCKET_DIR,
                Self::PLUGIN_NAME
            );
            let ipc = NipartIpcListener::new(&socket_path)?;
            log::debug!("Listening on {socket_path}");

            loop {
                if let Ok(conn) = ipc.accept().await {
                    log::debug!("Got daemon connection");
                    let plugin_clone = plugin.clone();
                    tokio::spawn(async move {
                        Self::process_connection(plugin_clone, conn).await
                    });
                }
            }
        }
    }

    fn process_connection(
        plugin: Arc<Self>,
        mut conn: NipartIpcConnection,
    ) -> impl Future<Output = Result<(), NipartError>> + Send {
        async move {
            loop {
                let cmd = conn.recv::<NipartPluginCmd>().await?;
                log::debug!("Got {cmd} from daemon");
                match cmd {
                    NipartPluginCmd::QueryPluginInfo => {
                        conn.send(Self::plugin_info(&plugin).await).await?
                    }
                    NipartPluginCmd::Quit => {
                        Self::quit(&plugin).await;
                    }
                    NipartPluginCmd::QueryNetworkState(opt) => {
                        let result =
                            Self::query_network_state(&plugin, *opt, &mut conn)
                                .await;
                        conn.send(result).await?
                    }
                    NipartPluginCmd::ApplyNetworkState(opt) => {
                        let (desired_state, opt) = *opt;
                        let result = Self::apply_network_state(
                            &plugin,
                            desired_state,
                            opt,
                            &mut conn,
                        )
                        .await;
                        conn.send(result).await?
                    }
                }
            }
        }
    }

    /// Return network state managed by this plugin only.
    /// Optionally, you may send log via `conn::log_debug()` and etc.
    /// Default implementation is return no support error.
    fn query_network_state(
        _plugin: &Arc<Self>,
        _opt: NipartstateQueryOption,
        _conn: &mut NipartIpcConnection,
    ) -> impl Future<Output = Result<NetworkState, NipartError>> + Send {
        async {
            Err(NipartError::new(
                ErrorKind::NoSupport,
                format!(
                    "Plugin {} has not implemented query_network_state()",
                    Self::PLUGIN_NAME
                ),
            ))
        }
    }

    /// Apply network state managed by this plugin only.
    /// Optionally, you may send log via `conn::log_debug()` and etc.
    fn apply_network_state(
        _plugin: &Arc<Self>,
        _desired_state: NetworkState,
        _opt: NipartstateApplyOption,
        _conn: &mut NipartIpcConnection,
    ) -> impl Future<Output = Result<(), NipartError>> + Send {
        async {
            Err(NipartError::new(
                ErrorKind::NoSupport,
                format!(
                    "Plugin {} has not implemented apply_network_state()",
                    Self::PLUGIN_NAME
                ),
            ))
        }
    }
}
