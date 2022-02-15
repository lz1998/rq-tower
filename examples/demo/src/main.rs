#![feature(async_closure)]

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use tokio::net::TcpStream;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use rq_tower::rq::device::Device;
use rq_tower::rq::version::{get_version, Protocol};
use rq_tower::rq::Client;
use rq_tower::rq::{LoginResponse, QRCodeState};
use rq_tower::service::builder::RQServiceBuilder;

use crate::handlers::print::{print_group, print_private};

mod handlers;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 打开日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .without_time(),
        )
        .with(
            tracing_subscriber::filter::Targets::new()
                .with_target("rs_qq", Level::DEBUG)
                .with_target("demo", Level::DEBUG),
        )
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

    // 创建 client
    let client = Arc::new(Client::new(
        load_device_or_random().await,
        get_version(Protocol::IPad),
        service,
    ));

    // 下面都是登录和自动重连，不用动

    // TCP 连接
    let stream = TcpStream::connect(SocketAddr::new(Ipv4Addr::new(42, 81, 176, 211).into(), 443))
        .await
        .expect("failed to connect tcp");
    // 开始处理网络
    let c = client.clone();
    let handle = tokio::spawn(async move { c.start_with_stream(stream).await });
    // 确保网络已开始处理
    tokio::task::yield_now().await;
    // 扫码登录，阻塞到登录成功
    qrcode_login(&client).await;
    // 登录成功后生成 token，用于掉线重连
    let token = client.gen_token().await;
    // 阻塞到掉线
    handle.await.ok();
    // 自动重连
    auto_reconnect(client, token, Duration::from_secs(10), 10).await;
}

// 扫码登录
async fn qrcode_login(client: &Arc<Client>) {
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
        tracing::info!("加载好友 {} 个", client.friends.read().await.len());
        client
            .reload_groups()
            .await
            .expect("failed to reload group list");
        tracing::info!("加载群 {} 个", client.groups.read().await.len());
    }
    start_heartbeat(client.clone()).await;
}

// 自动重连
async fn auto_reconnect(client: Arc<Client>, token: Bytes, interval: Duration, max: usize) {
    let mut count = 0;
    loop {
        client.stop();
        tracing::warn!("已掉线，10秒后重连，已重连 {} 次", count);
        tokio::time::sleep(interval).await;
        let stream = if let Ok(stream) =
            TcpStream::connect(SocketAddr::new(Ipv4Addr::new(42, 81, 176, 211).into(), 443)).await
        {
            count = 0;
            stream
        } else {
            count += 1;
            if count > max {
                break;
            }
            continue;
        };
        let c = client.clone();
        let handle = tokio::spawn(async move { c.start_with_stream(stream).await });
        tokio::task::yield_now().await; // 等一下，确保连上了
        if let Err(err) = client.token_login(token.clone()).await {
            // token 可能过期了
            tracing::error!("failed to token_login, err: {}", err);
            break;
        }
        client.register_client().await.unwrap();
        start_heartbeat(client.clone()).await;
        tracing::info!("掉线重连成功");
        handle.await.ok();
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

async fn load_device_or_random() -> Device {
    match Path::new("device.json").exists() {
        true => serde_json::from_str(
            &tokio::fs::read_to_string("device.json")
                .await
                .expect("failed to read device.json"),
        )
        .expect("failed to parse device info"),
        false => {
            let d = Device::random();
            tokio::fs::write("device.json", serde_json::to_string(&d).unwrap())
                .await
                .expect("failed to write device info to file");
            d
        }
    }
}
