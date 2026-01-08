// SPDX-License-Identifier: Apache-2.0

use futures_channel::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
    oneshot::{Sender, channel},
};
use futures_util::{SinkExt, StreamExt};
use nipart::{ErrorKind, NipartError};

pub(crate) trait TaskWorker: Sized + Send {
    type Cmd: std::fmt::Display + Send;
    type Reply: Send;
    // Once `associated_type_defaults` feature is stable, we should use this:
    // type FromManager = (Self::Cmd, Sender<Result<Self::Result, NipartError>>);

    #[allow(clippy::type_complexity)]
    fn new(
        receiver: UnboundedReceiver<(
            Self::Cmd,
            Sender<Result<Self::Reply, NipartError>>,
        )>,
    ) -> impl Future<Output = Result<Self, NipartError>> + Send;

    #[allow(clippy::type_complexity)]
    fn receiver(
        &mut self,
    ) -> &mut UnboundedReceiver<(Self::Cmd, Sender<Result<Self::Reply, NipartError>>)>;

    fn process_cmd(
        &mut self,
        cmd: Self::Cmd,
    ) -> impl Future<Output = Result<Self::Reply, NipartError>> + Send;

    #[allow(clippy::type_complexity)]
    fn recv_cmd(
        &mut self,
    ) -> impl Future<
        Output = Option<(Self::Cmd, Sender<Result<Self::Reply, NipartError>>)>,
    > + Send {
        async { self.receiver().next().await }
    }

    /// Default implementation of this function should be invoked in tokio
    /// worker thread.
    /// Return only when sender all dropped(daemon quit).
    fn run(&mut self) -> impl Future<Output = ()> + Send {
        async {
            loop {
                let (cmd, sender) = match self.recv_cmd().await {
                    Some(c) => c,
                    None => break,
                };
                let cmd_str = cmd.to_string();
                let result = self.process_cmd(cmd).await;
                if sender.send(result).is_err() {
                    log::error!("Failed to send reply for command {cmd_str}");
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TaskManager<C, R>
where
    C: std::fmt::Display + Clone,
{
    name: &'static str,
    sender: UnboundedSender<(C, Sender<Result<R, NipartError>>)>,
}

impl<C, R> TaskManager<C, R>
where
    C: std::fmt::Display + Clone,
{
    pub(crate) async fn new<W>(name: &'static str) -> Result<Self, NipartError>
    where
        W: TaskWorker<Cmd = C, Reply = R> + 'static,
    {
        let (sender, receiver) = unbounded::<(C, Sender<Result<R, NipartError>>)>();

        let mut worker = W::new(receiver).await?;

        tokio::spawn(async move { worker.run().await });

        Ok(Self { name, sender })
    }

    pub(crate) async fn exec(&mut self, cmd: C) -> Result<R, NipartError> {
        let (result_sender, result_receiver) = channel::<Result<R, NipartError>>();

        self.sender
            .send((cmd.clone(), result_sender))
            .await
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Manager {}: failed to send {}: {e}",
                        cmd, self.name
                    ),
                )
            })?;

        result_receiver.await.map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Manager {}: failed to receive reply for {cmd}: {e}",
                    self.name
                ),
            )
        })?
    }
}
