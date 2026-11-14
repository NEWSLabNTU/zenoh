use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use ordered_float::NotNan;
use petgraph::graph::NodeIndex;
use zenoh_buffers::writer::DidntWrite;
use zenoh_protocol::{core::ZenohIdProto, network::NetworkMessage};
use zenoh_task::TerminatableTask;
use zenoh_transport::unicast::TransportUnicast;

use super::{hat, hat_mut, HatTables};
use crate::net::{
    protocol::geolocation::{GeoLocation, GeoMotion},
    routing::dispatcher::{
        face::{Face, WeakFace},
        tables::TablesLock,
    },
};

type FaceMap = HashMap<ZenohIdProto, WeakFace>;

pub struct GeoLocationService {
    inner: Arc<GeoLocationServiceInner>,
    _task: TerminatableTask,
}

impl GeoLocationService {
    pub fn new_gpsd(announce_delay: Duration) -> Result<Self, Box<dyn Any + Send + Sync>> {
        let mut client = match gpsd_client::GPS::connect() {
            Ok(client) => client,
            Err(err) => return Err(Box::new(err)),
        };
        let inner = Arc::new(GeoLocationServiceInner::new());

        let task = TerminatableTask::spawn_abortable(zenoh_runtime::ZRuntime::Net, {
            let inner = inner.clone();

            async move {
                for sn in 0.. {
                    let data = loop {
                        tokio::time::sleep(announce_delay).await;

                        match client.current_data() {
                            Ok(data) => break data,
                            Err(err) => {
                                tracing::error!("fail to get data from GPSD: {err}");
                            }
                        }
                    };

                    let data = GeoLocationData {
                        sn,
                        lon: data.lon,
                        lat: data.lat,
                        motion: Some(GeoMotionData {
                            cog: data.track,
                            speed: data.speed,
                        }),
                    };

                    inner.save_geoloc(data);
                }
            }
        });
        Ok(Self { inner, _task: task })
    }

    pub fn new_fixed(announce_delay: Duration, lat: f64, lon: f64) -> Self {
        let inner = Arc::new(GeoLocationServiceInner::new());

        let task = TerminatableTask::spawn_abortable(zenoh_runtime::ZRuntime::Net, {
            let inner = inner.clone();

            async move {
                loop {
                    tokio::time::sleep(announce_delay).await;
                    let data = GeoLocationData {
                        sn: 0,
                        lon,
                        lat,
                        motion: None,
                    };

                    inner.save_geoloc(data);
                }
            }
        });

        Self { inner, _task: task }
    }

    pub fn add_face(&self, face: &Face) {
        self.inner.add_face(face);
    }

    pub fn remove_face(&self, zid: ZenohIdProto) {
        self.inner.remove_face(zid);
    }

    pub fn save_geolocs_from_from_oam(&self, geo_locations: Vec<GeoLocation>, src: ZenohIdProto) {
        self.inner.save_geolocs_from_from_oam(geo_locations, src);
    }
}

struct GeoLocationServiceInner {
    face_map: RwLock<FaceMap>,
    pending_zids: Mutex<HashSet<ZenohIdProto>>,
}

impl GeoLocationServiceInner {
    pub fn new() -> Self {
        let face_map = RwLock::new(HashMap::new());
        let pending_zids = Mutex::new(HashSet::new());
        Self {
            face_map,
            pending_zids,
        }
    }

    pub fn add_face(&self, face: &Face) {
        if let Some(tables) = self.tables() {
            assert!(
                std::ptr::eq(Arc::as_ptr(&tables), Arc::as_ptr(&face.tables)),
                "inserted face must bind to the same tables"
            );
        }
        zwrite!(self.face_map).insert(face.state.zid, face.downgrade());
    }

    pub fn remove_face(&self, zid: ZenohIdProto) {
        zwrite!(self.face_map)
            .remove(&zid)
            .unwrap_or_else(|| panic!("remoave a face with zid={zid} that was not added"));
    }

    pub fn save_geoloc(&self, data: GeoLocationData) {
        // Get the HatCode and self zid
        let Some(tables) = self.tables() else { return };
        let mut guard = zwrite!(tables.tables);
        let zid = guard.zid;
        let hat = hat_mut!(guard);

        // Store the data on the node pointed by zid
        let net = hat.linkstatepeers_net.as_mut().unwrap();
        let graph = &net.graph;
        let Some(idx) = graph.node_indices().find(|&idx| graph[idx].zid == zid) else {
            todo!();
        };
        let node = &mut net.graph[idx];
        node.geolocation = Some(data);

        // Schedule a future geo-location announcement
        self.schedule_geoloc_update([zid]);
    }

    pub(super) fn save_geolocs_from_from_oam(
        &self,
        mut geo_locations: Vec<GeoLocation>,
        src: ZenohIdProto,
    ) {
        // Get the network graph
        let Some(tables) = self.tables() else { return };
        let mut guard = zwrite!(tables.tables);
        let hat = hat_mut!(guard);
        let net = hat.linkstatepeers_net.as_mut().unwrap();

        let graph = &net.graph;

        let Some(src_link) = net.links.values_mut().find(|link| link.zid == src) else {
            tracing::error!(
                "{} Received GeoLocationList from unknown link {}",
                net.name,
                src
            );
            return;
        };

        let updated_zids = geo_locations.into_iter().filter_map(|msg| {
            // Map node psid to zid
            let zid = *src_link.get_zid(&msg.psid)?;
            let idx = net.get_idx(&zid)?;
            let node = &mut net.graph[idx];

            // Exclued those serial number is older
            if let Some(geo_loc) = &node.geolocation {
                if geo_loc.sn >= msg.sn {
                    return None;
                }
            };

            let GeoLocation {
                sn,
                lon,
                lat,
                motion,
                ..
            } = msg;

            let motion = motion.map(|m| GeoMotionData {
                cog: *m.cog,
                speed: *m.speed,
            });
            let data = GeoLocationData {
                sn,
                lon: *lon,
                lat: *lat,
                motion,
            };
            node.geolocation = Some(data);

            Some(zid)
        });

        self.schedule_geoloc_update(updated_zids);

        // net.links
        //     .values()
        //     .filter(|link| link.zid != src) // Exclude the link on the src
        //     .for_each(|curr_link| {
        // 	// Exclude node indexes which zid is the same with current's
        //         let filtered_idxes: Vec<_> = updated_idxes
        //             .iter()
        //             .filter(|(idx, _)| curr_link.zid != net.graph[*idx].zid)
        //             .cloned()
        //             .collect();

        // 	net.send_geoloc_on_link(filtered_idxes, &curr_link.transport);
        //     });

        // net.send_geoloc_on_link(updated_idxes, &src_link.transport);
    }

    pub fn schedule_geoloc_update<I>(&self, zids: I)
    where
        I: IntoIterator<Item = ZenohIdProto>,
    {
        let mut guard = zlock!(self.pending_zids);
        guard.extend(zids);
    }

    fn send_geoloc_on_link(&self, idxs: Vec<NodeIndex>, transport: &TransportUnicast) {
        if let Ok(msg) = self.make_geoloc_msg(&idxs) {
            tracing::trace!("{} Send to {:?} {:?}", self.name, transport.get_zid(), msg);
            if let Err(e) = transport.schedule(msg) {
                tracing::debug!("{} Error sending GeoLocationList: {}", self.name, e);
            }
        } else {
            tracing::error!("Failed to encode GeoLocation message");
        }
    }

    fn send_msg(&self, msg: GeoLocationData) {
        let Some(tables) = self.tables() else { return };
        let zids = {
            let mut guard = zlock!(self.pending_zids);
            std::mem::take(&mut *guard)
        };
        let guard = zread!(tables.tables);
        let hat = hat!(guard);

        todo!();
    }

    fn tables(&self) -> Option<Arc<TablesLock>> {
        let guard = zread!(self.face_map);
        let face = guard.values().next()?;
        let tables = face.tables.upgrade()?;
        Some(tables)
    }
}

#[derive(Clone)]
pub(super) struct GeoLocationData {
    pub(crate) sn: u64,
    pub(crate) lon: f64,
    pub(crate) lat: f64,
    pub(crate) motion: Option<GeoMotionData>,
}

#[derive(Clone)]
pub(super) struct GeoMotionData {
    pub(crate) cog: f32,
    pub(crate) speed: f32,
}
