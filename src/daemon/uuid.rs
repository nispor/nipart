// SPDX-License-Identifier: Apache-2.0

pub(crate) fn u128_to_uuid_string(id: u128) -> String {
    uuid::Uuid::from_u128(id).hyphenated().to_string()
}
