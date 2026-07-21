use std::{
    io::Write,
    net::{SocketAddr, TcpListener},
    thread,
    time::{Duration, Instant},
};

use open_edifier_core::DeviceEvent;
use open_edifier_s260::{ClientConfig, EventStream};

#[test]
fn event_stream_ignores_heartbeat_and_decodes_volume() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = response(0x003f, &[0; 9]);
        bytes.extend(response(0x0066, &[30, 18]));
        stream.write_all(&bytes).unwrap();
    });

    let mut events = EventStream::connect(config(address)).unwrap();
    assert_eq!(
        events.next_event(Duration::from_secs(2)).unwrap(),
        Some(DeviceEvent::Volume {
            current: 18,
            max: 30,
        })
    );
    server.join().unwrap();
}

#[test]
fn event_stream_reconnects_after_the_speaker_closes_the_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut first, _) = listener.accept().unwrap();
        first.write_all(&response(0x003f, &[0; 9])).unwrap();
        drop(first);

        let (mut second, _) = listener.accept().unwrap();
        second.write_all(&response(0x0066, &[30, 12])).unwrap();
    });

    let mut events = EventStream::connect(config(address)).unwrap();
    let event = events
        .next_event(Duration::from_secs(3))
        .unwrap()
        .expect("event stream did not reconnect");
    assert_eq!(
        event,
        DeviceEvent::Volume {
            current: 12,
            max: 30,
        }
    );
    server.join().unwrap();
}

#[test]
fn reconnect_backoff_waits_instead_of_busy_looping() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.write_all(&response(0x003f, &[0; 9])).unwrap();
        drop(stream);
        drop(listener);
    });

    let mut events = EventStream::connect(config(address)).unwrap();
    let started = Instant::now();
    assert!(events.next_event(Duration::from_millis(300)).is_err());
    assert!(started.elapsed() >= Duration::from_millis(250));
    server.join().unwrap();
}

fn response(command: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = vec![0xbb, 0xec];
    frame.extend(command.to_le_bytes());
    frame.push(payload.len() as u8);
    frame.extend(payload);
    let checksum = frame
        .iter()
        .fold(0_u8, |sum, value| sum.wrapping_add(*value));
    frame.push(checksum);
    frame
}

fn config(address: SocketAddr) -> ClientConfig {
    let mut config = ClientConfig::new(address.ip().to_string());
    config.port = address.port();
    config.connect_timeout = Duration::from_millis(100);
    config.request_timeout = Duration::from_secs(2);
    config
}
