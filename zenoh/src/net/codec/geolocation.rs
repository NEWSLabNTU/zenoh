use super::Zenoh080Routing;
use crate::net::protocol::geolocation::{self, GeoLocation};
use zenoh_buffers::{
    reader::{DidntRead, Reader},
    writer::{DidntWrite, Writer},
};
use zenoh_codec::{RCodec, WCodec, Zenoh080};
use zenoh_config::WhatAmI;
use zenoh_protocol::{common::imsg, core::ZenohIdProto};

// LinkState
impl<W> WCodec<&GeoLocation, &mut W> for Zenoh080Routing
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, msg: &GeoLocation) -> Self::Output {
        let codec = Zenoh080::new();
        // Options
        let mut options = 0;
        if msg.zid.is_some() {
            options |= geolocation::PID;
        }
        if msg.whatami.is_some() {
            options |= geolocation::WAI;
        }

        match (msg.cog.is_some(), msg.speed.is_some()) {
            (true, true) => {
                options |= geolocation::MOT;
            }
            (false, false) => {}
            _ => return Err(DidntWrite),
        }

        codec.write(&mut *writer, options)?;

        // Body
        codec.write(&mut *writer, msg.psid)?;
        codec.write(&mut *writer, msg.sn)?;
        if let Some(zid) = msg.zid.as_ref() {
            codec.write(&mut *writer, zid)?;
        }
        if let Some(wai) = msg.whatami {
            let wai: u8 = wai.into();
            codec.write(&mut *writer, wai)?;
        }

        codec.write(&mut *writer, msg.lon.to_le_bytes())?;
        codec.write(&mut *writer, msg.lat.to_le_bytes())?;
        if let Some(cog) = msg.cog {
            codec.write(&mut *writer, cog.to_le_bytes())?;
        }
        if let Some(speed) = msg.speed {
            codec.write(&mut *writer, speed.to_le_bytes())?;
        }

        Ok(())
    }
}

impl<R> RCodec<GeoLocation, &mut R> for Zenoh080Routing
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<GeoLocation, Self::Error> {
        macro_rules! read_f64 {
            ($codec:expr) => {{
                let bytes: [u8; 8] = $codec.read(&mut *reader)?;
                let float = f64::from_le_bytes(bytes);
                ordered_float::NotNan::new(float).map_err(|_| DidntRead)?
            }};
        }
        macro_rules! read_f32 {
            ($codec:expr) => {{
                let bytes: [u8; 4] = $codec.read(&mut *reader)?;
                let float = f32::from_le_bytes(bytes);
                ordered_float::NotNan::new(float).map_err(|_| DidntRead)?
            }};
        }

        let codec = Zenoh080::new();
        let options: u64 = codec.read(&mut *reader)?;
        let psid: u64 = codec.read(&mut *reader)?;
        let sn: u64 = codec.read(&mut *reader)?;
        let zid = if imsg::has_option(options, geolocation::PID) {
            let zid: ZenohIdProto = codec.read(&mut *reader)?;
            Some(zid)
        } else {
            None
        };
        let whatami = if imsg::has_option(options, geolocation::WAI) {
            let wai: u8 = codec.read(&mut *reader)?;
            Some(WhatAmI::try_from(wai).map_err(|_| DidntRead)?)
        } else {
            None
        };
        let lon = read_f64!(codec);
        let lat = read_f64!(codec);
        let (cog, speed) = if imsg::has_option(options, geolocation::MOT) {
            let cog = read_f32!(codec);
            let speed = read_f32!(codec);
            (Some(cog), Some(speed))
        } else {
            (None, None)
        };

        Ok(GeoLocation {
            psid,
            sn,
            zid,
            whatami,
            lon,
            lat,
            cog,
            speed,
        })
    }
}
