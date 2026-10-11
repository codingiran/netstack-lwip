//! Exercises the shared-ownership lifetime binding: the process-global lwIP
//! stack must stay alive until the *last* of NetStack / TcpListener / UdpSocket
//! (and any SendHalf) is dropped, regardless of drop order, and a fresh
//! NetStack must be creatable only once the previous one is fully gone.
//!
//! Everything is in a single test so the global `NETSTACK_ALIVE` guard isn't
//! raced by parallel tests.

use futures::{SinkExt, StreamExt};
use netstack_lwip::NetStack;

#[test]
fn stack_lifecycle_and_drop_order() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(10), run())
            .await
            .unwrap();
    });
}

async fn run() {
    let (stack, listener, udp) = NetStack::new().expect("create netstack");
    let local = udp.local_addr();
    assert!(local.ip().is_unspecified());
    assert_ne!(local.port(), 0);

    // Only one instance may exist at a time.
    assert!(
        NetStack::new().is_err(),
        "a second NetStack must fail while the first is alive"
    );

    // Dropping NetStack first must NOT tear down the shared stack: the listener
    // and udp socket still hold it, so re-creation stays blocked (and this must
    // not panic or deadlock).
    drop(stack);
    assert!(
        NetStack::new().is_err(),
        "stack still alive via listener/udp after NetStack dropped"
    );

    // A SendHalf alone must keep the stack alive after everything else is gone.
    let (send_half, recv_half) = udp.split();
    drop(listener);
    drop(recv_half);
    assert!(
        NetStack::new().is_err(),
        "stack still alive via a lone SendHalf"
    );

    // Dropping the final handle runs teardown synchronously (timer aborted,
    // callback cleared, guard released), so a fresh stack can now be built.
    drop(send_half);
    let (s2, l2, u2) = NetStack::new().expect("recreate after full teardown");
    drop((s2, l2, u2));

    // Exercise ownership through an accepted TCP stream, including repeated
    // teardown with a live connection (without waiting for a peer FIN).
    for _ in 0..16 {
        let (mut stack, mut listener, udp) = NetStack::new().unwrap();
        stack.send(tcp_packet(100, 0, 2)).await.unwrap();
        let syn_ack = stack.next().await.unwrap().unwrap();
        let server_sequence = u32::from_be_bytes(syn_ack[24..28].try_into().unwrap());
        stack
            .send(tcp_packet(101, server_sequence.wrapping_add(1), 16))
            .await
            .unwrap();
        let (stream, _, _) = listener.next().await.unwrap();
        drop((stack, listener, udp));
        assert!(
            NetStack::new().is_err(),
            "accepted TCP stream owns the stack"
        );
        drop(stream);
        let next = NetStack::new().expect("restart after active TCP stream dropped");
        drop(next);
    }
}

fn tcp_packet(sequence: u32, ack: u32, flags: u8) -> Vec<u8> {
    // IPv4 10.0.0.2:30000 -> 10.0.0.1:80; checksum validation is disabled in
    // this userspace stack. No OS network access or privileged TUN is needed.
    let mut packet = vec![0u8; 40];
    packet[0] = 0x45;
    packet[2..4].copy_from_slice(&40u16.to_be_bytes());
    packet[8] = 64;
    packet[9] = 6;
    packet[12..16].copy_from_slice(&[10, 0, 0, 2]);
    packet[16..20].copy_from_slice(&[10, 0, 0, 1]);
    packet[20..22].copy_from_slice(&30000u16.to_be_bytes());
    packet[22..24].copy_from_slice(&80u16.to_be_bytes());
    packet[24..28].copy_from_slice(&sequence.to_be_bytes());
    packet[28..32].copy_from_slice(&ack.to_be_bytes());
    packet[32] = 5 << 4;
    packet[33] = flags;
    packet[34..36].copy_from_slice(&65535u16.to_be_bytes());
    packet
}
