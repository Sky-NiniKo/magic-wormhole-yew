// Credit: https://github.com/andipabst/magic-wormhole-wasm/blob/main/src/lib.rs
use std::borrow::Cow;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use magic_wormhole::{Code, transfer, transit, Wormhole};


pub async fn send(file: web_sys::File) {
    match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
        Ok(file_content) => {
            let array = js_sys::Uint8Array::new(&file_content);
            let len = array.byte_length() as u64;
            let data_to_send: Vec<u8> = array.to_vec();
            log::debug!("Read raw data ({} bytes)", len);

            send_via_wormhole(
                data_to_send,
                len,
                file.name(),
            ).await
        }
        Err(_) => {
            log::error!("Error reading file");
        }
    }
}

struct NoOpFuture {}

impl Future for NoOpFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

// Credit: https://gitlab.com/lukas-heiligenbrunner/wormhole/-/blob/main/native/src/impls/helpers.rs
fn gen_relay_hints() -> Vec<transit::RelayHint> {
    let mut relay_hints: Vec<transit::RelayHint> = vec![];
    if relay_hints.is_empty() {
        relay_hints.push(
            transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()])
                .unwrap(),
        )
    }
    relay_hints
}
pub fn gen_app_config() -> magic_wormhole::AppConfig<transfer::AppVersion> {
    magic_wormhole::AppConfig {
        id: transfer::APPID,
        rendezvous_url: Cow::from(magic_wormhole::rendezvous::DEFAULT_RENDEZVOUS_SERVER),
        app_version: transfer::AppVersion {},
    }
}

async fn send_via_wormhole(file: Vec<u8>, file_size: u64, file_name: String) {
    let connect = Wormhole::connect_without_code(
        gen_app_config(),
        2,
    );

    match connect.await {
        Ok((server_welcome, connector)) => {
            log::info!("{}", server_welcome.code);

            match connector.await {
                Ok(wormhole) => {
                    let transfer_result = transfer::send_file(
                        wormhole,
                        gen_relay_hints(),
                        &mut &file[..],
                        PathBuf::from(file_name),
                        file_size,
                        transit::Abilities::ALL_ABILITIES,
                        |info| {
                            log::debug!("Connection type: '{:?}'", info.conn_type);
                        },
                        |cur, total| {
                            log::debug!("Progress: {}/{}", cur, total);
                        },
                        NoOpFuture {},
                    ).await;

                    match transfer_result {
                        Ok(_) => {
                            log::debug!("Data sent");
                        }
                        Err(e) => {
                            log::error!("Error in data transfer: {:?}", e);
                        }
                    }
                }
                Err(_) => {
                    log::error!("Error waiting for connection");
                }
            }
        }
        Err(e) => {
            log::error!("Error in connection: {}", e);
        }
    };
}

#[derive(Debug)]
struct ReceiveResult {
    data: Vec<u8>,
    filename: String,
    filesize: u64,
}

pub async fn receive(code: String) {
    let connect = Wormhole::connect_with_code(
        gen_app_config(),
        Code(code),
        true,
    );

    return match connect.await {
        Ok((_, wormhole)) => {
            let req = transfer::request_file(
                wormhole,
                gen_relay_hints(),
                transit::Abilities::ALL_ABILITIES,
                NoOpFuture {},
            ).await;

            let mut file: Vec<u8> = Vec::new();

            match req {
                Ok(Some(req)) => {
                    let filename = req.filename.clone();
                    let filesize = req.filesize;
                    log::info!("File name: {:?}, size: {}", filename, filesize);
                    let file_accept = req.accept(
                        |info| {
                            log::debug!("Connection type: '{:?}'", info.conn_type);
                        },
                        |cur, total| {
                            log::debug!("Progress: {}/{}", cur, total);
                        },
                        &mut file,
                        NoOpFuture {},
                    );

                    match file_accept.await {
                        Ok(_) => {
                            log::debug!("Data received, length: {}", file.len());
                            //let array: js_sys::Array = file.into_iter().map(JsValue::from).collect();
                            //data: js_sys::Uint8Array::new(&array),
                            let result = ReceiveResult {
                                data: file,
                                filename: filename.to_str().unwrap_or_default().into(),
                                filesize,
                            };
                            log::info!("Data receive: {:?}", result);
                        }
                        Err(e) => {
                            log::error!("Error in data transfer: {:?}", e);
                        }
                    }
                }
                _ => {
                    log::error!("No ReceiveRequest");
                }
            }
        }
        Err(e) => {
            log::error!("Error in connection: {}", e);
        }
    };
}