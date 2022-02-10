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

macro_rules! call_event {
    ($($ety: tt: $handler: tt),*) => {
        fn call(&mut self, e: QEvent) -> Self::Future {
            match e {
                $(
                    QEvent::$ety(e) => {
                        let mut handlers = self.$handler.clone();
                        Box::pin(async move {
                            for h in handlers.iter_mut() {
                                h.call(e.clone()).await.ok();
                            }
                            Ok(())
                        })
                    }
                )*
                _ => Box::pin(async { Ok(()) })
            }
        }
    };
}

impl Service<QEvent> for RQServiceBuilder {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    call_event!(
        Login: login_handlers,
        GroupMessage: group_message_handlers,
        PrivateMessage: private_message_handlers,
        GroupRequest: group_request_handlers,
        FriendRequest: friend_request_handlers
    );
}

macro_rules! on_event {
    ($fname: ident,$handler: tt, $aty: ty) => {
        pub fn $fname<F, Fut>(mut self, f: F) -> Self
        where
            F: Fn($aty) -> Fut + Copy + Send + Sync + 'static,
            Fut: Future<Output = ()> + Send,
        {
            let s: BoxCloneService<$aty, (), Infallible> = ServiceBuilder::new()
                .boxed_clone()
                .service_fn(move |req| async move {
                    f(req).await;
                    Ok(())
                });
            self.$handler.push(s);
            self
        }
    };
}

impl RQServiceBuilder {
    pub fn new() -> RQServiceBuilder {
        Self::default()
    }
    pub fn build(self) -> RQService {
        RQService(Buffer::new(self, 10))
    }

    on_event!(on_login, login_handlers, i64);
    on_event!(on_group_message, group_message_handlers, GroupMessageEvent);
    on_event!(
        on_private_message,
        private_message_handlers,
        PrivateMessageEvent
    );
    on_event!(on_group_request, group_request_handlers, GroupRequestEvent);
    on_event!(
        on_friend_request,
        friend_request_handlers,
        FriendRequestEvent
    );
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
