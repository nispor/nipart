# Managers of Daemon
 * `api`: Providing UNIX socket API to client.
 * `dhcp`: Managing DHCP.
 * `monitor`: Managing interface carrier monitoring.
 * `config`: Management the configuration.

# How managers communicate with each other.
 * Each manager holds the `Receiver` of mpsc channels for receiving message from
   other thread.
 * The `NmDaemonShareData` will hold the `Sender` part which can be safely
   cloned and move to any thread.
 * Any thread need reply from managers should send `Arc<Mutex<T>>` and waiting
   lock of that expecting manager finish its requested work and unlock that
   data.
