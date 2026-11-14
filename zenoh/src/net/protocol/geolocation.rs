use ordered_float::NotNan;
use zenoh_config::WhatAmI;
use zenoh_protocol::core::ZenohIdProto;

pub const WAI: u64 = 1 << 0; // 0x01
pub const MOT: u64 = 1 << 1; // 0x02

//  7 6 5 4 3 2 1 0
// +-+-+-+-+-+-+-+-+
// ~X|X|X|X|X|X|M|W~
// +-+-+-+-+-+-+-+-+
// ~     psid      ~
// +---------------+
// ~      sn       ~
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
    pub(crate) whatami: Option<WhatAmI>,
    pub(crate) lon: NotNan<f64>,
    pub(crate) lat: NotNan<f64>,
    pub(crate) motion: Option<GeoMotion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeoMotion {
    /// Course over ground.
    pub(crate) cog: NotNan<f32>,
    pub(crate) speed: NotNan<f32>,
}

//  7 6 5 4 3 2 1 0
// +-+-+-+-+-+-+-+-+
// |X|X|X|GEOLOC_LS|
// +-+-+-+---------+
// ~[geo_locations] ~
// +---------------+
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeoLocationList {
    pub(crate) geo_locations: Vec<GeoLocation>,
}
