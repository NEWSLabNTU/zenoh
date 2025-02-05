use ordered_float::NotNan;
use zenoh_config::WhatAmI;
use zenoh_protocol::core::ZenohIdProto;

pub const PID: u64 = 1; // 0x01
pub const WAI: u64 = 1 << 1; // 0x02
pub const MOT: u64 = 1 << 2; // 0x04

//  7 6 5 4 3 2 1 0
// +-+-+-+-+-+-+-+-+
// ~X|X|X|X|X|M|W|P~
// +-+-+-+-+-+-+-+-+
// ~     psid      ~
// +---------------+
// ~      sn       ~
// +---------------+
// ~      zid      ~ if P == 1
// +---------------+
// ~    whatami    ~ if W == 1
// +---------------+
// ~      lon      ~
// +---------------+
// ~      lat      ~
// +---------------+
// ~  cog  | speed ~ if M == 1
// +---------------+
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeoLocation {
    pub(crate) psid: u64,
    pub(crate) sn: u64,
    pub(crate) zid: Option<ZenohIdProto>,
    pub(crate) whatami: Option<WhatAmI>,
    pub(crate) lon: NotNan<f64>,
    pub(crate) lat: NotNan<f64>,
    /// Course over ground.
    pub(crate) cog: Option<NotNan<f32>>,
    pub(crate) speed: Option<NotNan<f32>>,
}
