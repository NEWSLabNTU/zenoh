use super::Zenoh080Routing;
use crate::net::protocol::geolocation::{self, GeoLocation, GeoLocationList, GeoMotion};
use zenoh_buffers::{
    reader::{DidntRead, Reader},
    writer::{DidntWrite, Writer},
};
use zenoh_codec::{RCodec, WCodec, Zenoh080};
use zenoh_config::WhatAmI;
use zenoh_protocol::common::imsg;

// GeoLocation
impl<W> WCodec<&GeoLocation, &mut W> for Zenoh080Routing
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, msg: &GeoLocation) -> Self::Output {
        let codec = Zenoh080::new();
        // Options
        let mut options = 0;
        if msg.whatami.is_some() {
            options |= geolocation::WAI;
        }

        if msg.motion.is_some() {
            options |= geolocation::MOT;
        }

        codec.write(&mut *writer, options)?;

        // Body
        codec.write(&mut *writer, msg.psid)?;
        codec.write(&mut *writer, msg.sn)?;
        if let Some(wai) = msg.whatami {
            let wai: u8 = wai.into();
            codec.write(&mut *writer, wai)?;
        }

        codec.write(&mut *writer, msg.lon.to_le_bytes())?;
        codec.write(&mut *writer, msg.lat.to_le_bytes())?;
        if let Some(motion) = &msg.motion {
            codec.write(&mut *writer, motion.cog.to_le_bytes())?;
            codec.write(&mut *writer, motion.speed.to_le_bytes())?;
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
        let whatami = if imsg::has_option(options, geolocation::WAI) {
            let wai: u8 = codec.read(&mut *reader)?;
            Some(WhatAmI::try_from(wai).map_err(|_| DidntRead)?)
        } else {
            None
        };
        let lon = read_f64!(codec);
        let lat = read_f64!(codec);
        let motion = if imsg::has_option(options, geolocation::MOT) {
            let cog = read_f32!(codec);
            let speed = read_f32!(codec);
            Some(GeoMotion { cog, speed })
        } else {
            None
        };

        Ok(GeoLocation {
            psid,
            sn,
            whatami,
            lon,
            lat,
            motion,
        })
    }
}

impl<W> WCodec<&GeoLocationList, &mut W> for Zenoh080Routing
where
    W: Writer,
{
    type Output = Result<(), DidntWrite>;

    fn write(self, writer: &mut W, x: &GeoLocationList) -> Self::Output {
        let codec = Zenoh080::new();

        codec.write(&mut *writer, x.geo_locations.len())?;
        for ls in &x.geo_locations {
            self.write(&mut *writer, ls)?;
        }

        Ok(())
    }
}

impl<R> RCodec<GeoLocationList, &mut R> for Zenoh080Routing
where
    R: Reader,
{
    type Error = DidntRead;

    fn read(self, reader: &mut R) -> Result<GeoLocationList, Self::Error> {
        let codec = Zenoh080::new();

        let len: usize = codec.read(&mut *reader)?;
        let mut geo_locations = Vec::with_capacity(len);
        for _ in 0..len {
            let ls: GeoLocation = self.read(&mut *reader)?;
            geo_locations.push(ls);
        }

        Ok(GeoLocationList { geo_locations })
    }
}
