use rs_qq::client::event::{
    FriendRequestEvent, GroupMessageEvent, GroupRequestEvent, PrivateMessageEvent,
};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tower::buffer::Buffer;
use tower::util::BoxCloneService;
use tower::{Service, ServiceBuilder};

use crate::rq::handler::QEvent;
use crate::service::RQService;

#[derive(Default, Clone)]
pub struct RQServiceBuilder {
    login_handlers: Vec<BoxCloneService<i64, (), Infallible>>,
    group_message_handlers: Vec<BoxCloneService<GroupMessageEvent, (), Infallible>>,
    private_message_handlers: Vec<BoxCloneService<PrivateMessageEvent, (), Infallible>>,
    group_request_handlers: Vec<BoxCloneService<GroupRequestEvent, (), Infallible>>,
    friend_request_handlers: Vec<BoxCloneService<FriendRequestEvent, (), Infallible>>,
}

impl Service<QEvent> for RQServiceBuilder {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: QEvent) -> Self::Future {
        // let mut handlers = self.login_handlers.clone();
        return match req {
            QEvent::LoginEvent(uin) => {
                let mut handlers = self.login_handlers.clone();
                Box::pin(async move {
                    for h in handlers.iter_mut() {
                        h.call(uin).await.ok();
                    }
                    Ok(())
                })
            }
            QEvent::GroupMessage(event) => {
                let mut handlers = self.group_message_handlers.clone();
                Box::pin(async move {
                    for h in handlers.iter_mut() {
                        h.call(event.clone()).await.ok();
                    }
                    Ok(())
                })
            }
            QEvent::PrivateMessage(event) => {
                let mut handlers = self.private_message_handlers.clone();
                Box::pin(async move {
                    for h in handlers.iter_mut() {
                        h.call(event.clone()).await.ok();
                    }
                    Ok(())
                })
            }
            QEvent::GroupRequest(event) => {
                let mut handlers = self.group_request_handlers.clone();
                Box::pin(async move {
                    for h in handlers.iter_mut() {
                        h.call(event.clone()).await.ok();
                    }
                    Ok(())
                })
            }
            QEvent::FriendRequest(event) => {
                let mut handlers = self.friend_request_handlers.clone();
                Box::pin(async move {
                    for h in handlers.iter_mut() {
                        h.call(event.clone()).await.ok();
                    }
                    Ok(())
                })
            }
            _ => Box::pin(async move { Ok(()) }),
        };
    }
}

impl RQServiceBuilder {
    pub fn new() -> RQServiceBuilder {
        Self::default()
    }
    pub fn build(self) -> RQService {
        RQService(Buffer::new(self, 10))
    }
    pub fn on_login<F, Fut>(mut self, f: F) -> Self
    where
        F: FnOnce(i64) -> Fut + Copy + Send + 'static + Sync,
        Fut: Future<Output = ()> + Send,
    {
        let s: BoxCloneService<i64, (), Infallible> = ServiceBuilder::new()
            .boxed_clone()
            .service_fn(move |req| async move {
                f(req).await;
                Ok(())
            });
        self.login_handlers.push(s);
        self
    }

    pub fn on_group_message<F, Fut>(mut self, f: F) -> Self
    where
        F: FnOnce(GroupMessageEvent) -> Fut + Copy + Send + 'static + Sync,
        Fut: Future<Output = ()> + Send,
    {
        let s: BoxCloneService<GroupMessageEvent, (), Infallible> = ServiceBuilder::new()
            .boxed_clone()
            .service_fn(move |req| async move {
                f(req).await;
                Ok(())
            });
        self.group_message_handlers.push(s);
        self
    }
    pub fn on_private_message<F, Fut>(mut self, f: F) -> Self
    where
        F: FnOnce(PrivateMessageEvent) -> Fut + Copy + Send + 'static + Sync,
        Fut: Future<Output = ()> + Send,
    {
        let s: BoxCloneService<PrivateMessageEvent, (), Infallible> = ServiceBuilder::new()
            .boxed_clone()
            .service_fn(move |req| async move {
                f(req).await;
                Ok(())
            });
        self.private_message_handlers.push(s);
        self
    }

    pub fn on_group_request<F, Fut>(mut self, f: F) -> Self
    where
        F: FnOnce(GroupRequestEvent) -> Fut + Copy + Send + 'static + Sync,
        Fut: Future<Output = ()> + Send,
    {
        let s: BoxCloneService<GroupRequestEvent, (), Infallible> = ServiceBuilder::new()
            .boxed_clone()
            .service_fn(move |req| async move {
                f(req).await;
                Ok(())
            });
        self.group_request_handlers.push(s);
        self
    }

    pub fn on_friend_request<F, Fut>(mut self, f: F) -> Self
    where
        F: FnOnce(FriendRequestEvent) -> Fut + Copy + Send + 'static + Sync,
        Fut: Future<Output = ()> + Send,
    {
        let s: BoxCloneService<FriendRequestEvent, (), Infallible> = ServiceBuilder::new()
            .boxed_clone()
            .service_fn(move |req| async move {
                f(req).await;
                Ok(())
            });
        self.friend_request_handlers.push(s);
        self
    }
}

impl Service<i64> for RQServiceBuilder {
    type Response = i64;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: i64) -> Self::Future {
        let fut = async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok(req + 1)
        };
        Box::pin(fut)
    }
}
