// tower = { version = "0.4.11", default-features = false, features = ["util", "buffer", "make", "timeout"] }
// tokio = { version = "1", features = ["full"] }

use async_trait::async_trait;
use tower::buffer::Buffer;
use tower::{Service, ServiceExt};

use crate::rq::handler::{Handler, QEvent};
use crate::service::builder::RQServiceBuilder;

pub mod builder;
#[async_trait]
impl Handler for RQService {
    async fn handle(&self, msg: QEvent) {
        let mut svc = self.0.clone();
        svc.ready()
            .await
            .expect("RQService crashed")
            .call(msg)
            .await
            .ok();
    }
}

pub struct RQService(Buffer<RQServiceBuilder, QEvent>);

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tower::buffer::Buffer;
    use tower::{Service, ServiceBuilder, ServiceExt};

    use super::*;

    #[tokio::test]
    async fn t() {
        let svc = ServiceBuilder::new()
            .concurrency_limit(1)
            .service(RQServiceBuilder::default());
        let svc = Buffer::new(svc, 10);
        for i in 0..10 {
            let mut svc = svc.clone();
            tokio::spawn(async move {
                let a = svc.ready().await.expect("crashed").call(i).await;
                eprintln!("{:?}", a);
            });
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
