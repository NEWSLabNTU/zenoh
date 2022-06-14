//
// Copyright (c) 2022 ZettaScale Technology
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
use super::face::{Face, FaceState};
use super::id::FID;
use super::network::{shared_nodes, Network};
pub use super::pubsub::*;
pub use super::queries::*;
pub use super::resource::*;
use super::runtime::Runtime;
use async_std::task::JoinHandle;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};
use std::sync::{Mutex, RwLock};
use std::time::Duration;
use uhlc::HLC;
use zenoh_link::Link;
use zenoh_protocol::proto::{ZenohBody, ZenohMessage};
use zenoh_protocol_core::{whatami::WhatAmI::*, PeerId, WhatAmI, ZInt};
use zenoh_transport::{DeMux, Mux, Primitives, TransportPeerEventHandler, TransportUnicast};
// use zenoh_collections::Timer;
use zenoh_core::zconfigurable;
use zenoh_core::Result as ZResult;
use zenoh_sync::get_mut_unchecked;

zconfigurable! {
    static ref TREES_COMPUTATION_DELAY: u64 = 100;
}

pub struct Tables {
    pub(crate) pid: PeerId,
    pub(crate) whatami: WhatAmI,
    face_counter: usize,
    #[allow(dead_code)]
    pub(crate) hlc: Option<Arc<HLC>>,
    // pub(crate) timer: Timer,
    // pub(crate) queries_default_timeout: Duration,
    pub(crate) root_res: Arc<Resource>,
    pub(crate) faces: HashMap<usize, Arc<FaceState>>,
    pub(crate) pull_caches_lock: Mutex<()>,
    pub(crate) router_subs: HashSet<Arc<Resource>>,
    pub(crate) peer_subs: HashSet<Arc<Resource>>,
    pub(crate) router_qabls: HashSet<Arc<Resource>>,
    pub(crate) peer_qabls: HashSet<Arc<Resource>>,
    pub(crate) routers_net: Option<Network>,
    pub(crate) peers_net: Option<Network>,
    pub(crate) shared_nodes: Vec<PeerId>,
    pub(crate) routers_trees_task: Option<JoinHandle<()>>,
    pub(crate) peers_trees_task: Option<JoinHandle<()>>,
}

impl Tables {
    pub fn new(
        pid: PeerId,
        whatami: WhatAmI,
        hlc: Option<Arc<HLC>>,
        _queries_default_timeout: Duration,
    ) -> Self {
        Tables {
            pid,
            whatami,
            face_counter: 0,
            hlc,
            // timer: Timer::new(true),
            // queries_default_timeout,
            root_res: Resource::root(),
            faces: HashMap::new(),
            pull_caches_lock: Mutex::new(()),
            router_subs: HashSet::new(),
            peer_subs: HashSet::new(),
            router_qabls: HashSet::new(),
            peer_qabls: HashSet::new(),
            routers_net: None,
            peers_net: None,
            shared_nodes: vec![],
            routers_trees_task: None,
            peers_trees_task: None,
        }
    }

    #[doc(hidden)]
    pub fn _get_root(&self) -> &Arc<Resource> {
        &self.root_res
    }

    pub fn print(&self) -> String {
        Resource::print_tree(&self.root_res)
    }

    #[inline]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(crate) fn get_mapping<'a>(
        &'a self,
        face: &'a FaceState,
        expr_id: &ZInt,
    ) -> Option<&'a Arc<Resource>> {
        match expr_id {
            0 => Some(&self.root_res),
            expr_id => face.get_mapping(expr_id),
        }
    }

    #[inline]
    pub(crate) fn get_net(&self, net_type: WhatAmI) -> Option<&Network> {
        match net_type {
            Router => self.routers_net.as_ref(),
            Peer => self.peers_net.as_ref(),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn get_face(&self, fid: FID) -> Option<&FaceState> {
        todo!();
    }

    fn open_net_face(
        &mut self,
        pid: PeerId,
        whatami: WhatAmI,
        primitives: Arc<dyn Primitives + Send + Sync>,
        link_id: usize,
    ) -> Weak<FaceState> {
        let fid = self.face_counter;
        self.face_counter += 1;
        let mut newface = self
            .faces
            .entry(fid)
            .or_insert_with(|| FaceState::new(fid, pid, whatami, primitives.clone(), link_id))
            .clone();
        log::debug!("New {}", newface);

        pubsub_new_face(self, &mut newface);
        queries_new_face(self, &mut newface);

        Arc::downgrade(&newface)
    }

    pub fn open_face(
        &mut self,
        pid: PeerId,
        whatami: WhatAmI,
        primitives: Arc<dyn Primitives + Send + Sync>,
    ) -> Weak<FaceState> {
        self.open_net_face(pid, whatami, primitives, 0)
    }

    pub fn close_face(&mut self, face: FID) {
        match face.upgrade() {
            Some(mut face) => {
                log::debug!("Close {}", face);
                finalize_pending_queries(self, &mut face);

                let mut face_clone = face.clone();
                let face = get_mut_unchecked(&mut face);
                for res in face.remote_mappings.values_mut() {
                    get_mut_unchecked(res).session_ctxs.remove(&face.id);
                    Resource::clean(res);
                }
                face.remote_mappings.clear();
                for res in face.local_mappings.values_mut() {
                    get_mut_unchecked(res).session_ctxs.remove(&face.id);
                    Resource::clean(res);
                }
                face.local_mappings.clear();
                for mut res in face.remote_subs.drain() {
                    get_mut_unchecked(&mut res).session_ctxs.remove(&face.id);
                    undeclare_client_subscription(self, &mut face_clone, &mut res);
                    Resource::clean(&mut res);
                }
                for (mut res, kind) in face.remote_qabls.drain() {
                    get_mut_unchecked(&mut res).session_ctxs.remove(&face.id);
                    undeclare_client_queryable(self, &mut face_clone, &mut res, kind);
                    Resource::clean(&mut res);
                }
                self.faces.remove(&face.id);
            }
            None => log::error!("Face already closed!"),
        }
    }

    fn compute_routes(&mut self, res: &mut Arc<Resource>) {
        compute_data_routes(self, res);
        compute_query_routes(self, res);
    }

    pub(crate) fn compute_matches_routes(&mut self, res: &mut Arc<Resource>) {
        if res.context.is_some() {
            self.compute_routes(res);

            let resclone = res.clone();
            for match_ in &mut get_mut_unchecked(res).context_mut().matches {
                let match_ = &mut match_.upgrade().unwrap();
                if !Arc::ptr_eq(match_, &resclone) && match_.context.is_some() {
                    self.compute_routes(match_);
                }
            }
        }
    }

    pub(crate) fn schedule_compute_trees(
        &mut self,
        tables_ref: Arc<RwLock<Tables>>,
        net_type: WhatAmI,
    ) {
        log::trace!("Schedule computations");
        if (net_type == Router && self.routers_trees_task.is_none())
            || (net_type == Peer && self.peers_trees_task.is_none())
        {
            let task = Some(async_std::task::spawn(async move {
                async_std::task::sleep(std::time::Duration::from_millis(*TREES_COMPUTATION_DELAY))
                    .await;
                let mut tables = zwrite!(tables_ref);

                log::trace!("Compute trees");
                let new_childs = match net_type {
                    Router => tables.routers_net.as_mut().unwrap().compute_trees(),
                    _ => tables.peers_net.as_mut().unwrap().compute_trees(),
                };

                log::trace!("Compute routes");
                pubsub_tree_change(&mut tables, &new_childs, net_type);
                queries_tree_change(&mut tables, &new_childs, net_type);

                log::trace!("Computations completed");
                match net_type {
                    Router => tables.routers_trees_task = None,
                    _ => tables.peers_trees_task = None,
                };
            }));
            match net_type {
                Router => self.routers_trees_task = task,
                _ => self.peers_trees_task = task,
            };
        }
    }
}

pub struct Router {
    whatami: WhatAmI,
    pub tables: Arc<RwLock<Tables>>,
}

impl Router {
    pub fn new(
        pid: PeerId,
        whatami: WhatAmI,
        hlc: Option<Arc<HLC>>,
        queries_default_timeout: Duration,
    ) -> Self {
        Router {
            whatami,
            tables: Arc::new(RwLock::new(Tables::new(
                pid,
                whatami,
                hlc,
                queries_default_timeout,
            ))),
        }
    }

    pub fn init_link_state(
        &mut self,
        runtime: Runtime,
        peers_autoconnect: bool,
        routers_autoconnect_gossip: bool,
    ) {
        let mut tables = zwrite!(self.tables);
        tables.peers_net = Some(Network::new(
            "[Peers network]".to_string(),
            tables.pid,
            runtime.clone(),
            peers_autoconnect,
            routers_autoconnect_gossip,
        ));
        if runtime.whatami == Router {
            tables.routers_net = Some(Network::new(
                "[Routers network]".to_string(),
                tables.pid,
                runtime,
                peers_autoconnect,
                routers_autoconnect_gossip,
            ));
            tables.shared_nodes = shared_nodes(
                tables.routers_net.as_ref().unwrap(),
                tables.peers_net.as_ref().unwrap(),
            );
        }
    }

    pub fn new_primitives(&self, primitives: Arc<dyn Primitives + Send + Sync>) -> Arc<Face> {
        Arc::new(Face {
            tables: self.tables.clone(),
            state: {
                let mut tables = zwrite!(self.tables);
                let pid = tables.pid;
                tables.open_face(pid, Client, primitives).upgrade().unwrap()
            },
        })
    }

    pub fn new_transport_unicast(
        &self,
        transport: TransportUnicast,
    ) -> ZResult<Arc<LinkStateInterceptor>> {
        let mut tables = zwrite!(self.tables);
        let whatami = transport.get_whatami()?;

        let link_id = match (self.whatami, whatami) {
            (Router, Router) => tables
                .routers_net
                .as_mut()
                .unwrap()
                .add_link(transport.clone()),
            (Router, Peer) | (Peer, Router) | (Peer, Peer) => tables
                .peers_net
                .as_mut()
                .unwrap()
                .add_link(transport.clone()),
            _ => 0,
        };

        if tables.whatami == Router {
            tables.shared_nodes = shared_nodes(
                tables.routers_net.as_ref().unwrap(),
                tables.peers_net.as_ref().unwrap(),
            );
        }

        let handler = Arc::new(LinkStateInterceptor::new(
            transport.clone(),
            self.tables.clone(),
            Face {
                tables: self.tables.clone(),
                state: tables
                    .open_net_face(
                        transport.get_pid().unwrap(),
                        whatami,
                        Arc::new(Mux::new(transport)),
                        link_id,
                    )
                    .upgrade()
                    .unwrap(),
            },
        ));

        match (self.whatami, whatami) {
            (Router, Router) => {
                tables.schedule_compute_trees(self.tables.clone(), Router);
            }
            (Router, Peer) | (Peer, Router) | (Peer, Peer) => {
                tables.schedule_compute_trees(self.tables.clone(), Peer);
            }
            _ => (),
        }
        Ok(handler)
    }
}

pub struct LinkStateInterceptor {
    pub(crate) transport: TransportUnicast,
    pub(crate) tables: Arc<RwLock<Tables>>,
    pub(crate) face: Face,
    pub(crate) demux: DeMux<Face>,
}

impl LinkStateInterceptor {
    fn new(transport: TransportUnicast, tables: Arc<RwLock<Tables>>, face: Face) -> Self {
        LinkStateInterceptor {
            transport,
            tables,
            face: face.clone(),
            demux: DeMux::new(face),
        }
    }
}

impl TransportPeerEventHandler for LinkStateInterceptor {
    fn handle_message(&self, msg: ZenohMessage) -> ZResult<()> {
        use WhatAmI as W;

        log::trace!("Recv {:?}", msg);

        match msg.body {
            ZenohBody::LinkStateList(list) => {
                let pid = if let Ok(pid) = self.transport.get_pid() {
                    pid
                } else {
                    return Ok(());
                };

                let mut tables = zwrite!(self.tables);
                let whatami = self.transport.get_whatami()?;
                match (tables.whatami, whatami) {
                    (W::Router, W::Router) => {
                        for (_, removed_node) in tables
                            .routers_net
                            .as_mut()
                            .unwrap()
                            .link_states(list.link_states, pid)
                        {
                            pubsub_remove_node(&mut tables, &removed_node.pid, W::Router);
                            queries_remove_node(&mut tables, &removed_node.pid, W::Router);
                        }

                        tables.shared_nodes = shared_nodes(
                            tables.routers_net.as_ref().unwrap(),
                            tables.peers_net.as_ref().unwrap(),
                        );

                        tables.schedule_compute_trees(self.tables.clone(), W::Router);
                    }
                    (W::Router, W::Peer) | (W::Peer, W::Router) | (W::Peer, W::Peer) => {
                        for (_, removed_node) in tables
                            .peers_net
                            .as_mut()
                            .unwrap()
                            .link_states(list.link_states, pid)
                        {
                            pubsub_remove_node(&mut tables, &removed_node.pid, W::Peer);
                            queries_remove_node(&mut tables, &removed_node.pid, W::Peer);
                        }

                        if tables.whatami == W::Router {
                            tables.shared_nodes = shared_nodes(
                                tables.routers_net.as_ref().unwrap(),
                                tables.peers_net.as_ref().unwrap(),
                            );
                        }

                        tables.schedule_compute_trees(self.tables.clone(), W::Peer);
                    }
                    _ => (),
                };

                Ok(())
            }
            _ => self.demux.handle_message(msg),
        }
    }

    fn new_link(&self, _link: Link) {}

    fn del_link(&self, _link: Link) {}

    fn closing(&self) {
        self.demux.closing();
        let tables_ref = self.tables.clone();
        match (self.transport.get_pid(), self.transport.get_whatami()) {
            (Ok(pid), Ok(whatami)) => {
                let mut tables = zwrite!(tables_ref);
                match (tables.whatami, whatami) {
                    (Router, Router) => {
                        for (_, removed_node) in
                            tables.routers_net.as_mut().unwrap().remove_link(&pid)
                        {
                            pubsub_remove_node(&mut tables, &removed_node.pid, Router);
                            queries_remove_node(&mut tables, &removed_node.pid, Router);
                        }

                        tables.shared_nodes = shared_nodes(
                            tables.routers_net.as_ref().unwrap(),
                            tables.peers_net.as_ref().unwrap(),
                        );

                        tables.schedule_compute_trees(tables_ref.clone(), Router);
                    }
                    (Router, Peer) | (Peer, Router) | (Peer, Peer) => {
                        for (_, removed_node) in
                            tables.peers_net.as_mut().unwrap().remove_link(&pid)
                        {
                            pubsub_remove_node(&mut tables, &removed_node.pid, Peer);
                            queries_remove_node(&mut tables, &removed_node.pid, Peer);
                        }

                        if tables.whatami == Router {
                            tables.shared_nodes = shared_nodes(
                                tables.routers_net.as_ref().unwrap(),
                                tables.peers_net.as_ref().unwrap(),
                            );
                        }

                        tables.schedule_compute_trees(tables_ref.clone(), Peer);
                    }
                    _ => (),
                };
            }
            (_, _) => log::error!("Closed transport in session closing!"),
        }
    }

    fn closed(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
