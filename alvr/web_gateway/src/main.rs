use std::time::Duration;

use alvr_client_core::ClientCoreEvent;
use alvr_common::glam::{Quat, UVec2, Vec3};
use futures_util::{stream::SplitSink, SinkExt};
use warp::filters::ws::{Message, WebSocket};

#[repr(u8)]
enum WebSocketBinaryServerToClientMessagePrefix {
    FrameReady = 1,
    CreateDecoder = 2,
}

#[repr(u8)]
enum WebSocketBinaryClientToServerMessagePrefix {
    HeadMountDisplayTracking = 1,
}

async fn alvr_to_websocket(websocket: &mut SplitSink<WebSocket, Message>) {
    loop {
        let event = alvr_client_core::poll_event();
        let Some(event) = event else {
            match websocket.flush().await {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("websocket flush error: {}", e);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
            continue;
        };

        let result = match event {
            ClientCoreEvent::UpdateHudMessage(message) => {
                websocket.send(Message::text(message)).await
            },
            ClientCoreEvent::StreamingStarted { view_resolution, refresh_rate_hint, settings } => {
                println!("streaming started {:?}, {}", view_resolution, refresh_rate_hint);
                alvr_client_core::request_idr();
                Ok(())
            },
            ClientCoreEvent::StreamingStopped => {
                println!("streaming stopped");
                Ok(())
            },
            ClientCoreEvent::Haptics { device_id, duration, frequency, amplitude } => {
                println!("haptics {:?} {:?} {:?} {:?}", device_id, duration, frequency, amplitude);
                Ok(())
            },
            ClientCoreEvent::MaybeCreateDecoder { codec, config_nal } => {
                println!("maybe create decoder {:?} {:?}", codec, config_nal.len());
                let mut msg = vec![0u8; config_nal.len() + 8];
                msg[0..4].copy_from_slice(&(WebSocketBinaryServerToClientMessagePrefix::CreateDecoder as u32).to_le_bytes());
                msg[4..8].copy_from_slice(&(codec as u8 as u32).to_le_bytes());
                msg[8..].copy_from_slice(&config_nal);

                websocket.send(Message::binary(msg)).await
            },
            ClientCoreEvent::FrameReady { timestamp, nal } => {
                println!("frame ready {:?} {:?}", timestamp, nal.len());
                let mut msg = vec![0u8; nal.len() + (8 * 3)];
                msg[0..1].copy_from_slice(&(WebSocketBinaryServerToClientMessagePrefix::FrameReady as u8).to_le_bytes());
                msg[8..24].copy_from_slice(&timestamp.as_micros().to_le_bytes());
                msg[24..].copy_from_slice(&nal);
                alvr_client_core::report_frame_decoded(timestamp);

                websocket.send(Message::binary(msg)).await
            }
        };

        match result {
            Ok(_) => {},
            Err(e) => {
                eprintln!("websocket send error: {}", e);
                break;
            }
        }
    }

    std::process::exit(0);
}

async fn websocket_handler(websocket: WebSocket) {
    use futures_util::stream::StreamExt;
    use futures_util::SinkExt;
    let (mut tx, mut rx) = websocket.split();
    tokio::spawn(async move {
        alvr_to_websocket(&mut tx).await;
    });
    let mut old_config: Option<([alvr_common::Fov; 2], f32)> = None;
    while let Some(result) = rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error: {}", e);
                break;
            }
        };

        if msg.is_text() {
            let text = msg.to_str().unwrap();
            if text == "hello" {
                let resolution = UVec2::new(1680, 1760);
                alvr_client_core::initialize(
                    resolution,
                    vec![90.0],
                    true,
                );
                alvr_client_core::resume();
            } else if text == "idr" {
                alvr_client_core::request_idr();
            } else if text.starts_with("decoded:") {
                let ts = text[8..].parse::<u64>().unwrap();
                alvr_client_core::report_frame_decoded(Duration::from_micros(ts));
            } else if text.starts_with("alive:") {
                let ts = text[6..].parse::<u64>().unwrap();
            } else {
                println!("unknown text: {}", text)
            }
        } else if msg.is_binary() {
            let data = msg.as_bytes();
            let prefix = u32::from_le_bytes(data[0..4].try_into().unwrap());
            let timestamp = u64::from_le_bytes(data[4..12].try_into().unwrap());
            if prefix == (WebSocketBinaryClientToServerMessagePrefix::HeadMountDisplayTracking as u8 as u32) {
                let config = ([
                    alvr_common::Fov {
                        left: f32::from_le_bytes(data[64..68].try_into().unwrap()),
                        right: f32::from_le_bytes(data[68..72].try_into().unwrap()),
                        up: f32::from_le_bytes(data[72..76].try_into().unwrap()),
                        down: f32::from_le_bytes(data[76..80].try_into().unwrap()),
                    },
                    alvr_common::Fov {
                        left: f32::from_le_bytes(data[80..84].try_into().unwrap()),
                        right: f32::from_le_bytes(data[84..88].try_into().unwrap()),
                        up: f32::from_le_bytes(data[88..92].try_into().unwrap()),
                        down: f32::from_le_bytes(data[92..96].try_into().unwrap()),
                    }
                ], 0.063f32);
                // println!("config: l.l={}, l.r={}, l.u={}, l.d={}, r.l={}, r.r={}, r.u={}, r.d={}", config.0[0].left, config.0[0].right, config.0[0].up, config.0[0].down, config.0[1].left, config.0[1].right, config.0[1].up, config.0[1].down);
                if old_config != Some(config) {
                    println!("config:    {}          {}", config.0[0].up, config.0[1].up);
                    println!("    {}           {} | {}           {}", config.0[0].left, config.0[0].right, config.0[1].left, config.0[1].right);
                    println!("           {}          {}", config.0[0].down, config.0[1].down);
                    alvr_client_core::send_views_config(config.0, config.1);
                    old_config = Some(config);
                }

                let pose = alvr_common::DeviceMotion {
                    pose: alvr_common::Pose {
                        orientation: Quat {
                            x: f32::from_le_bytes(data[12..16].try_into().unwrap()),
                            y: f32::from_le_bytes(data[16..20].try_into().unwrap()),
                            z: f32::from_le_bytes(data[20..24].try_into().unwrap()),
                            w: f32::from_le_bytes(data[24..28].try_into().unwrap())
                        }, 
                        position: Vec3 {
                            x: f32::from_le_bytes(data[28..32].try_into().unwrap()),
                            y: f32::from_le_bytes(data[32..36].try_into().unwrap()),
                            z: f32::from_le_bytes(data[36..40].try_into().unwrap())
                        }
                    },
                    linear_velocity: Vec3 {
                        x: f32::from_le_bytes(data[40..44].try_into().unwrap()),
                        y: f32::from_le_bytes(data[44..48].try_into().unwrap()),
                        z: f32::from_le_bytes(data[48..52].try_into().unwrap())
                    },
                    angular_velocity: Vec3 {
                        x: f32::from_le_bytes(data[52..56].try_into().unwrap()),
                        y: f32::from_le_bytes(data[56..60].try_into().unwrap()),
                        z: f32::from_le_bytes(data[60..64].try_into().unwrap())
                    },
                };
                // println!("pose: {:?}", pose);
                alvr_client_core::send_tracking(alvr_packets::Tracking {
                    target_timestamp: Duration::from_micros(timestamp),
                    device_motions: vec![
                        (alvr_common::hash_string("/user/head"), pose)
                    ],
                    hand_skeletons: [None, None],
                    face_data: alvr_packets::FaceData { eye_gazes: [None, None], fb_face_expression: None, htc_eye_expression: None, htc_lip_expression: None },
                });

            } else {
                println!("unknown binary: {:?}", data)
            }
        } else if msg.is_close() {
            println!("received close");
        } else {
            println!("received unknown: {:?}", msg);
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Hello!");
    
    use warp::Filter;

    let hello = warp::path("websocket")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(websocket_handler)
        });
    warp::serve(hello).run(([0, 0, 0, 0], 5999)).await;
}
