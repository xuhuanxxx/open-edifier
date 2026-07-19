use std::{
    io::{Read, Write},
    net::TcpListener,
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

    let mut client = Client::connect(ClientConfig {
        host: address.ip().to_string(),
        port: address.port(),
        timeout: Duration::from_secs(2),
    })
    .unwrap();

    assert_eq!(client.status().unwrap().source, Source::new(Source::USB));
    let updated = client.set_source(Source::new(Source::AUX)).unwrap();
    assert_eq!(updated.source, Source::new(Source::AUX));
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

    let mut client = Client::connect(ClientConfig {
        host: address.ip().to_string(),
        port: address.port(),
        timeout: Duration::from_secs(2),
    })
    .unwrap();
    assert!(matches!(client.status(), Err(Error::Protocol(message)) if message.contains("code")));
    server.join().unwrap();
}

#[test]
fn rejects_a_zero_connection_timeout() {
    let result = Client::connect(ClientConfig {
        host: "127.0.0.1".to_owned(),
        port: 9,
        timeout: Duration::ZERO,
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

    let mut client = Client::connect(ClientConfig {
        host: address.ip().to_string(),
        port: address.port(),
        timeout: Duration::from_secs(2),
    })
    .unwrap();
    assert_eq!(
        client.set_eq_preset(1).unwrap().equalizer.unwrap().preset,
        1
    );
    client.playback(PlaybackAction::Pause).unwrap();
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
            "volume": 18,
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
