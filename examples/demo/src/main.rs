#![feature(async_closure)]

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;

use rq_tower::rq::device::Device;
use rq_tower::rq::version::{get_version, Protocol};
use rq_tower::rq::Client;
use rq_tower::rq::{LoginResponse, QRCodeState};
use rq_tower::service::builder::RQServiceBuilder;

use crate::handlers::print::{print_group, print_private};

mod handlers;

#[tokio::main]
async fn main() {
    // 构造 tower service
    let service = RQServiceBuilder::new()
        .on_group_message(print_group)
        .on_private_message(print_private)
        .on_group_request(async move |e| {
            println!("{:?}", e.request);
            e.accept().await.ok();
        })
        .on_friend_request(async move |e| {
            println!("{:?}", e.request);
            e.accept().await.ok();
        })
        .build();
    let cli = Client::new(Device::random(), get_version(Protocol::IPad), service);

    // 下面都是登录流程，不用动
    let client = Arc::new(cli);
    let c = client.clone();
    let handle = tokio::spawn(async move {
        c.start().await.expect("failed to run client");
    });
    tokio::time::sleep(Duration::from_millis(200)).await; // 等一下，确保连上了
    let mut resp = client.fetch_qrcode().await.expect("failed to fetch qrcode");
    let mut image_sig = Bytes::new();
    loop {
        match resp {
            QRCodeState::QRCodeImageFetch {
                ref image_data,
                ref sig,
            } => {
                tokio::fs::write("qrcode.png", &image_data)
                    .await
                    .expect("failed to write file");
                image_sig = sig.clone();
                println!("二维码: qrcode.png");
            }
            QRCodeState::QRCodeWaitingForScan => {
                println!("二维码待扫描")
            }
            QRCodeState::QRCodeWaitingForConfirm => {
                println!("二维码待确认")
            }
            QRCodeState::QRCodeTimeout => {
                println!("二维码已超时，重新获取");
                if let QRCodeState::QRCodeImageFetch {
                    ref image_data,
                    ref sig,
                } = client.fetch_qrcode().await.expect("failed to fetch qrcode")
                {
                    tokio::fs::write("qrcode.png", &image_data)
                        .await
                        .expect("failed to write file");
                    image_sig = sig.clone();
                    println!("二维码: qrcode.png");
                }
            }
            QRCodeState::QRCodeConfirmed {
                ref tmp_pwd,
                ref tmp_no_pic_sig,
                ref tgt_qr,
                ..
            } => {
                println!("二维码已确认");
                let mut login_resp = client
                    .qrcode_login(tmp_pwd, tmp_no_pic_sig, tgt_qr)
                    .await
                    .expect("failed to qrcode login");
                if let LoginResponse::DeviceLockLogin { .. } = login_resp {
                    login_resp = client
                        .device_lock_login()
                        .await
                        .expect("failed to device lock login");
                }
                println!("{:?}", login_resp);
                break;
            }
            QRCodeState::QRCodeCanceled => {
                panic!("二维码已取消")
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        resp = client
            .query_qrcode_result(&image_sig)
            .await
            .expect("failed to query qrcode result");
    }
    client.register_client().await.unwrap();
    {
        client
            .reload_friends()
            .await
            .expect("failed to reload friend list");
        println!("{:?}", client.friends.read().await);
        client
            .reload_groups()
            .await
            .expect("failed to reload group list");
        println!("{:?}", client.groups.read().await);
    }
    handle.await.ok();
}
