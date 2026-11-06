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
//! Over a transport whose only link can lose messages (a lone `udp/` link), a publisher
//! must keep reaching its subscribers when its key expression declaration is lost.
use std::{
    net::{SocketAddr, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use zenoh::Session;
use zenoh_core::ztimeout;
use zenoh_test::{get_free_tcp_port, get_free_udp_port};

const TIMEOUT: Duration = Duration::from_secs(60);

/// Relays datagrams between one client and `server`, dropping the client's datagrams
/// while `drop` is set.
fn lossy_relay(server: u16, drop: Arc<AtomicBool>) -> u16 {
    let front = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = front.local_addr().unwrap().port();
    let back = UdpSocket::bind("127.0.0.1:0").unwrap();
    back.connect(("127.0.0.1", server)).unwrap();
    let client: Arc<std::sync::Mutex<Option<SocketAddr>>> = Default::default();
    {
        let (front, back, client) = (
            front.try_clone().unwrap(),
            back.try_clone().unwrap(),
            client.clone(),
        );
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 65536];
            while let Ok((n, from)) = front.recv_from(&mut buf) {
                *client.lock().unwrap() = Some(from);
                if !drop.load(Ordering::SeqCst) {
                    let _ = back.send(&buf[..n]);
                }
            }
        });
    }
    std::thread::spawn(move || {
        let mut buf = vec![0u8; 65536];
        while let Ok(n) = back.recv(&mut buf) {
            if let Some(to) = *client.lock().unwrap() {
                let _ = front.send_to(&buf[..n], to);
            }
        }
    });
    port
}

async fn open(mode: &str, listen: &[String], connect: &[String]) -> Session {
    let mut config = zenoh::Config::default();
    config.insert_json5("mode", &format!("{mode:?}")).unwrap();
    config
        .insert_json5("listen/endpoints", &serde_json::to_string(listen).unwrap())
        .unwrap();
    config
        .insert_json5(
            "connect/endpoints",
            &serde_json::to_string(connect).unwrap(),
        )
        .unwrap();
    config
        .insert_json5("scouting/multicast/enabled", "false")
        .unwrap();
    ztimeout!(zenoh::open(config)).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn udp_lost_keyexpr_declaration() {
    zenoh_util::init_log_from_env_or("error");
    let udp = get_free_udp_port();
    let tcp = get_free_tcp_port();
    let router = open(
        "router",
        &[
            format!("udp/127.0.0.1:{udp}"),
            format!("tcp/127.0.0.1:{tcp}"),
        ],
        &[],
    )
    .await;

    let subscriber = open("client", &[], &[format!("tcp/127.0.0.1:{tcp}")]).await;
    let sub = ztimeout!(subscriber.declare_subscriber("test/udp/**")).unwrap();

    let drop = Arc::new(AtomicBool::new(false));
    let relay = lossy_relay(udp, drop.clone());
    let publisher = open("client", &[], &[format!("udp/127.0.0.1:{relay}")]).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The publisher's declarations are lost.
    drop.store(true, Ordering::SeqCst);
    let publ = ztimeout!(publisher.declare_publisher("test/udp/value")).unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    drop.store(false, Ordering::SeqCst);

    for i in 0..20 {
        ztimeout!(publ.put(format!("{i}"))).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut received = 0;
    while let Ok(Some(_)) = sub.try_recv() {
        received += 1;
    }
    assert!(
        received > 0,
        "no put reached the subscriber after the declaration was lost"
    );

    ztimeout!(publisher.close()).unwrap();
    ztimeout!(subscriber.close()).unwrap();
    ztimeout!(router.close()).unwrap();
}
