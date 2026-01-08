// SPDX-License-Identifier: Apache-2.0

const UDEV_DB_DIR: &str = "/run/udev/data";

// As systemd `src/libsystemd/sd-device/sd-device.c` function
// `device_read_db_internal_filename()` comment says:
//      devices with a database entry are initialized
//
// And base on code of systemd `device_get_id_filename()` function, the
// database file for network interface is:
//      /run/udev/data/n{iface_index}
pub(crate) fn udev_net_device_is_initialized(iface_index: u32) -> bool {
    std::path::Path::new(&format!("{UDEV_DB_DIR}/n{iface_index}")).exists()
}
