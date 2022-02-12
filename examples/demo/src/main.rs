#![feature(async_closure)]

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use tokio::task::JoinHandle;

use rq_tower::rq::device::Device;
use rq_tower::rq::version::{get_version, Protocol};
use rq_tower::rq::{Client, RQResult};
use rq_tower::rq::{LoginResponse, QRCodeState};
use rq_tower::service::builder::RQServiceBuilder;

use crate::handlers::print::{print_group, print_private};

mod handlers;

#[tokio::main]
async fn main() {
    // 打开日志
    let env = tracing_subscriber::EnvFilter::from("rs_qq=debug,info");
    tracing_subscriber::fmt()
        .with_env_filter(env)
        .without_time()
        .init();

    // 构造 tower service
    let service = RQServiceBuilder::new()
        .on_group_message(print_group)
        .on_private_message(print_private)
        .on_group_request(async move |e| {
            tracing::info!("{:?}", e.request);
            e.accept().await.ok();
        })
        .on_friend_request(async move |e| {
            tracing::info!("{:?}", e.request);
            e.accept().await.ok();
        })
        .build();
    let cli = Client::new(Device::random(), get_version(Protocol::IPad), service);

    // 下面都是登录流程，不用动
    let client = Arc::new(cli);
    let handle = qrcode_login(client.clone()).await;
    let token = client.gen_token().await;
    handle.await.ok(); // 阻塞到掉线
    auto_reconnect(client, token, Duration::from_secs(10), 10).await;
}

// 扫码登录
async fn qrcode_login(client: Arc<Client>) -> JoinHandle<RQResult<()>> {
    let c = client.clone();
    let handle = tokio::spawn(async move { c.start().await });
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
                tracing::info!("二维码: qrcode.png");
            }
            QRCodeState::QRCodeWaitingForScan => {
                tracing::info!("二维码待扫描")
            }
            QRCodeState::QRCodeWaitingForConfirm => {
                tracing::info!("二维码待确认")
            }
            QRCodeState::QRCodeTimeout => {
                tracing::info!("二维码已超时，重新获取");
                if let QRCodeState::QRCodeImageFetch {
                    ref image_data,
                    ref sig,
                } = client.fetch_qrcode().await.expect("failed to fetch qrcode")
                {
                    tokio::fs::write("qrcode.png", &image_data)
                        .await
                        .expect("failed to write file");
                    image_sig = sig.clone();
                    tracing::info!("二维码: qrcode.png");
                }
            }
            QRCodeState::QRCodeConfirmed {
                ref tmp_pwd,
                ref tmp_no_pic_sig,
                ref tgt_qr,
                ..
            } => {
                tracing::info!("二维码已确认");
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
                tracing::info!("{:?}", login_resp);
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
        tracing::info!("{:?}", client.friends.read().await);
        client
            .reload_groups()
            .await
            .expect("failed to reload group list");
        tracing::info!("{:?}", client.groups.read().await);
    }
    start_heartbeat(client).await;
    handle
}

// 自动重连
async fn auto_reconnect(client: Arc<Client>, token: Bytes, interval: Duration, max: usize) {
    let mut count = 0;
    loop {
        tracing::warn!("已掉线，10秒后重连");
        tokio::time::sleep(interval).await;
        let c = client.clone();
        let handle = tokio::spawn(async move { c.start().await });
        tokio::time::sleep(Duration::from_millis(200)).await; // 等一下，确保连上了
        if let Err(err) = client.token_login(token.clone()).await {
            tracing::error!("failed to token_login, err: {}", err);
            client.stop();
            count += 1;
            if count > max {
                break;
            }
            continue;
        } else {
            count = 0;
            client.register_client().await;
            start_heartbeat(client.clone()).await;
            tracing::info!("掉线重连成功");
        }
        if let Err(err) = handle.await {
            tracing::error!("disconnect: {}", err);
        }
    }
}

// 开启心跳，重连之前开了就不开
async fn start_heartbeat(client: Arc<Client>) {
    if !client.heartbeat_enabled.load(Ordering::Relaxed) {
        tokio::spawn(async move {
            client.do_heartbeat().await;
        });
    }
}
