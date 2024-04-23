//
// Copyright (c) 2023 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//

//! ⚠️ WARNING ⚠️
//!
//! This crate is intended for Zenoh's internal use.
//!
//! [Click here for Zenoh's documentation](../zenoh/index.html)
mod multicast;
mod unicast;

use async_trait::async_trait;
pub use multicast::*;
use std::net::SocketAddr;
pub use unicast::*;
use zenoh_core::zconfigurable;
use zenoh_link_commons::LocatorInspector;
use zenoh_protocol::core:: Locator;
use zenoh_result::ZResult;

pub const BTP_LOCATOR_PREFIX: &str = "btp";

/////////// We may set distance in this configuration in the future//////////////
zconfigurable! {
    // Default MTU (UDP PDU) in bytes.
    //static ref UDP_DEFAULT_MTU: u16 = UDP_MTU_LIMIT;
    // Amount of time in microseconds to throttle the accept loop upon an error.
    // Default set to 100 ms.
    //static ref UDP_ACCEPT_THROTTLE_TIME: u64 = 100_000;
}

#[derive(Default, Clone, Copy)]
pub struct BtpLocatorInspector;
#[async_trait]
impl LocatorInspector for BtpLocatorInspector {
    fn protocol(&self) -> &str {
        BTP_LOCATOR_PREFIX
    }
    // Directly return ture. we only use broadcast
    async fn is_multicast(&self, locator: &Locator) -> ZResult<bool> {
        Ok(true)
    }
}

///////////// It is used to assign interface, /////////////////
// pub mod config {
//     pub const UDP_MULTICAST_IFACE: &str = "iface";
//     pub const UDP_MULTICAST_JOIN: &str = "join";
// }

////////////// I do not know whether it is important or not. It can be fulfilled with mac address in the future /////////////////
// pub async fn get_udp_addrs(address: Address<'_>) -> ZResult<impl Iterator<Item = SocketAddr>> {
//     let iter = address
//         .as_str()
//         .to_socket_addrs()
//         .await
//         .map_err(|e| zerror!("{}", e))?;
//     Ok(iter)
// }

/////////////I do not know whether it is necessary or not /////////////////
// pub(crate) fn socket_addr_to_udp_locator(addr: &SocketAddr) -> Locator {
//     Locator::new(BTP_LOCATOR_PREFIX, addr.to_string(), "").unwrap()
// }
