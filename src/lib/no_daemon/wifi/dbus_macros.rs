// SPDX-License-Identifier: Apache-2.0

macro_rules! _from_map {
    ($map: ident, $remove: expr, $convert: expr) => {
        $map.remove($remove).map($convert).transpose().map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Invalid wpa_supplicant DBUS reply of `{}` property: {e}",
                    $remove
                ),
            )
        })
    };
}
