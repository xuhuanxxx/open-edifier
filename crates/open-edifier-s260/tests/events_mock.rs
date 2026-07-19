use std::{io::Write, net::TcpListener, thread, time::Duration};

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

    let mut events = EventStream::connect(ClientConfig {
        host: address.ip().to_string(),
        port: address.port(),
        timeout: Duration::from_secs(2),
    })
    .unwrap();
    assert_eq!(
        events.next_event().unwrap(),
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

    let mut events = EventStream::connect(ClientConfig {
        host: address.ip().to_string(),
        port: address.port(),
        timeout: Duration::from_secs(2),
    })
    .unwrap();

    assert_eq!(events.next_event().unwrap(), None);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let event = loop {
        if let Some(event) = events.next_event().unwrap() {
            break event;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "event stream did not reconnect"
        );
        thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(
        event,
        DeviceEvent::Volume {
            current: 12,
            max: 30,
        }
    );
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
