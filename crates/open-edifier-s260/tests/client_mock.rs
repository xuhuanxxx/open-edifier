use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    thread,
    time::Duration,
};

use open_edifier_core::PlaybackAction;
use open_edifier_s260::{Client, ClientConfig, Error, Source};
use serde_json::{Value, json};

#[test]
fn status_and_source_change_are_verified_end_to_end() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut source = 2_u8;
        for request_number in 0..4 {
            let mut bytes = [0_u8; 2048];
            let size = stream.read(&mut bytes).unwrap();
            let request: Value = serde_json::from_slice(&bytes[..size]).unwrap();
            let id = request["id"].as_str().unwrap();

            let response = if request["payload"] == "settings" {
                source = request["inputSource"]["selectedIndex"].as_u64().unwrap() as u8;
                json!({"code": 0, "id": id, "payload": "settings", "message": "success"})
            } else {
                status(id, source)
            };

            let heartbeat = if request_number == 0 {
                &[0xbb, 0xec, 0x3f, 0x00, 0x09][..]
            } else {
                &[]
            };
            stream.write_all(&frame(&response, heartbeat)).unwrap();
        }
    });

    let mut client = Client::connect(config(address)).unwrap();

    assert_eq!(
        client.status().unwrap().source,
        Some(Source::new(Source::USB))
    );
    let updated = client.set_source(Source::new(Source::AUX)).unwrap();
    assert_eq!(updated.source, Some(Source::new(Source::AUX)));
    server.join().unwrap();
}

#[test]
fn rejects_a_response_without_an_explicit_result_code() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0_u8; 2048];
        let size = stream.read(&mut bytes).unwrap();
        let request: Value = serde_json::from_slice(&bytes[..size]).unwrap();
        let response = json!({
            "id": request["id"],
            "payload": "status_query",
            "message": "success"
        });
        stream.write_all(&frame(&response, &[])).unwrap();
    });

    let mut client = Client::connect(config(address)).unwrap();
    assert!(matches!(client.status(), Err(Error::Protocol(message)) if message.contains("code")));
    server.join().unwrap();
}

#[test]
fn rejected_errors_do_not_include_untrusted_response_fields() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0_u8; 2048];
        let size = stream.read(&mut bytes).unwrap();
        let request: Value = serde_json::from_slice(&bytes[..size]).unwrap();
        let response = json!({
            "code": 7,
            "id": request["id"],
            "payload": "status_query",
            "message": "rejected",
            "wifiName": "private-network",
            "bluetoothPairingRecord": [{"name": "private-device"}]
        });
        stream.write_all(&frame(&response, &[])).unwrap();
    });

    let mut client = Client::connect(config(address)).unwrap();
    let error = client.status().unwrap_err();
    assert!(matches!(
        error,
        Error::Rejected {
            code: 7,
            ref message
        } if message == "rejected"
    ));
    let displayed = error.to_string();
    assert!(!displayed.contains("private-network"));
    assert!(!displayed.contains("private-device"));
    server.join().unwrap();
}

#[test]
fn rejects_a_zero_connection_timeout() {
    let result = Client::connect(ClientConfig {
        host: "127.0.0.1".to_owned(),
        port: 9,
        connect_timeout: Duration::ZERO,
        request_timeout: Duration::from_secs(1),
        verification_timeout: Duration::from_secs(1),
        verification_interval: Duration::from_millis(50),
    });
    assert!(
        matches!(result, Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::InvalidInput)
    );
}

#[test]
fn eq_and_playback_commands_use_verified_json_fields() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut eq = 0_u8;
        for _ in 0..4 {
            let mut bytes = [0_u8; 2048];
            let size = stream.read(&mut bytes).unwrap();
            let request: Value = serde_json::from_slice(&bytes[..size]).unwrap();
            let id = request["id"].as_str().unwrap();
            let response = if request["payload"] == "settings" {
                if let Some(preset) = request["soundEffect"]["selectedIndex"].as_u64() {
                    eq = preset as u8;
                } else {
                    assert_eq!(request["player"]["playerStatus"], 0);
                }
                json!({"code": 0, "id": id, "payload": "settings", "message": "success"})
            } else {
                status_with_eq(id, 2, eq)
            };
            stream.write_all(&frame(&response, &[])).unwrap();
        }
    });

    let mut client = Client::connect(config(address)).unwrap();
    assert_eq!(
        client.set_eq_preset(1).unwrap().equalizer.unwrap().preset,
        1
    );
    client.playback(PlaybackAction::Pause).unwrap();
    server.join().unwrap();
}

#[test]
fn volume_verification_retries_until_the_device_reports_the_target() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut status_reads = 0;
        for _ in 0..4 {
            let mut bytes = [0_u8; 2048];
            let size = stream.read(&mut bytes).unwrap();
            let request: Value = serde_json::from_slice(&bytes[..size]).unwrap();
            let id = request["id"].as_str().unwrap();
            let response = if request["payload"] == "settings" {
                assert_eq!(request["player"]["volume"], 19);
                json!({"code": 0, "id": id, "payload": "settings", "message": "success"})
            } else {
                status_reads += 1;
                status_with_volume(id, 2, 0, if status_reads < 3 { 18 } else { 19 })
            };
            stream.write_all(&frame(&response, &[])).unwrap();
        }
    });

    let mut client = Client::connect(config(address)).unwrap();
    let updated = client.set_volume(19).unwrap();
    assert_eq!(updated.volume.unwrap().current, 19);
    server.join().unwrap();
}

#[test]
fn volume_verification_has_a_bounded_structured_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut status_reads = 0_u8;
        loop {
            let mut bytes = [0_u8; 2048];
            let size = match stream.read(&mut bytes) {
                Ok(size) => size,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::ConnectionReset
                    ) =>
                {
                    break;
                }
                Err(error) => panic!("mock server read failed: {error}"),
            };
            if size == 0 {
                break;
            }
            let request: Value = serde_json::from_slice(&bytes[..size]).unwrap();
            let id = request["id"].as_str().unwrap();
            let response = if request["payload"] == "settings" {
                json!({"code": 0, "id": id, "payload": "settings", "message": "success"})
            } else {
                status_reads += 1;
                if status_reads >= 3 {
                    thread::sleep(Duration::from_millis(150));
                }
                status_with_volume(id, 2, 0, 18)
            };
            if stream.write_all(&frame(&response, &[])).is_err() {
                break;
            }
        }
    });

    let mut client_config = config(address);
    client_config.verification_timeout = Duration::from_millis(120);
    client_config.verification_interval = Duration::from_millis(20);
    let mut client = Client::connect(client_config).unwrap();
    assert!(matches!(
        client.set_volume(19),
        Err(Error::VerificationTimeout {
            field: "volume",
            expected,
            actual,
            attempts,
            ..
        }) if expected == "19" && actual == "18" && attempts >= 2
    ));
    drop(client);
    server.join().unwrap();
}

fn frame(value: &Value, prefix: &[u8]) -> Vec<u8> {
    let payload = serde_json::to_vec(value).unwrap();
    let mut result = prefix.to_vec();
    result.extend([0xee, 0xdd, 0xff, 0xee]);
    result.extend((payload.len() as u16).to_be_bytes());
    result.extend(payload);
    result
}

fn status(id: &str, source: u8) -> Value {
    status_with_eq(id, source, 0)
}

fn status_with_eq(id: &str, source: u8, eq: u8) -> Value {
    status_with_volume(id, source, eq, 18)
}

fn status_with_volume(id: &str, source: u8, eq: u8, volume: u8) -> Value {
    json!({
        "code": 0,
        "id": id,
        "payload": "status_query",
        "message": "success",
        "supportedFeatures": ["deviceInfo", "inputSource", "player"],
        "deviceInfo": {
            "bluetoothName": "EDIFIER S260",
            "firmwareVersion": "01.00.00"
        },
        "inputSource": {
            "inputIndex": 1,
            "selectedIndex": source
        },
        "player": {
            "volume": volume,
            "minVolume": 0,
            "maxVolume": 30,
            "playerStatus": 1
        },
        "soundEffect": {
            "selectedIndex": eq,
            "soundIndex": 3
        }
    })
}

fn config(address: SocketAddr) -> ClientConfig {
    let mut config = ClientConfig::new(address.ip().to_string());
    config.port = address.port();
    config.connect_timeout = Duration::from_secs(2);
    config.request_timeout = Duration::from_secs(2);
    config
}
