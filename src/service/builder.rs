use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use rs_qq::client::event::{
    DeleteFriendEvent, FriendMessageRecallEvent, FriendPokeEvent, FriendRequestEvent,
    GroupLeaveEvent, GroupMessageEvent, GroupMessageRecallEvent, GroupMuteEvent,
    GroupNameUpdateEvent, GroupRequestEvent, MemberPermissionChangeEvent, NewFriendEvent,
    NewMemberEvent, PrivateMessageEvent, SelfInvitedEvent,
};
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
    self_invited_handlers: Vec<BoxCloneService<SelfInvitedEvent, (), Infallible>>,
    new_member_handlers: Vec<BoxCloneService<NewMemberEvent, (), Infallible>>,
    group_mute_handlers: Vec<BoxCloneService<GroupMuteEvent, (), Infallible>>,
    friend_message_recall_handlers: Vec<BoxCloneService<FriendMessageRecallEvent, (), Infallible>>,
    group_message_recall_handlers: Vec<BoxCloneService<GroupMessageRecallEvent, (), Infallible>>,
    new_friend_handlers: Vec<BoxCloneService<NewFriendEvent, (), Infallible>>,
    group_leave_handlers: Vec<BoxCloneService<GroupLeaveEvent, (), Infallible>>,
    friend_poke_handlers: Vec<BoxCloneService<FriendPokeEvent, (), Infallible>>,
    group_name_update_handlers: Vec<BoxCloneService<GroupNameUpdateEvent, (), Infallible>>,
    delete_friend_handlers: Vec<BoxCloneService<DeleteFriendEvent, (), Infallible>>,
    member_permission_change_handlers:
        Vec<BoxCloneService<MemberPermissionChangeEvent, (), Infallible>>,
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

#[allow(clippy::type_complexity)]
impl Service<QEvent> for RQServiceBuilder {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    call_event!(
        LoginEvent: login_handlers,
        GroupMessage: group_message_handlers,
        PrivateMessage: private_message_handlers,
        GroupRequest: group_request_handlers,
        SelfInvited: self_invited_handlers,
        FriendRequest: friend_request_handlers,
        NewMember: new_member_handlers,
        GroupMute: group_mute_handlers,
        FriendMessageRecall: friend_message_recall_handlers,
        GroupMessageRecall: group_message_recall_handlers,
        NewFriend: new_friend_handlers,
        GroupLeave: group_leave_handlers,
        FriendPoke: friend_poke_handlers,
        GroupNameUpdate: group_name_update_handlers,
        DeleteFriend: delete_friend_handlers,
        MemberPermissionChange: member_permission_change_handlers
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
    on_event!(on_self_invited, self_invited_handlers, SelfInvitedEvent);
    on_event!(
        on_friend_request,
        friend_request_handlers,
        FriendRequestEvent
    );
    on_event!(on_new_member, new_member_handlers, NewMemberEvent);
    on_event!(on_group_mute, group_mute_handlers, GroupMuteEvent);
    on_event!(
        on_friend_message_recall,
        friend_message_recall_handlers,
        FriendMessageRecallEvent
    );
    on_event!(
        on_group_message_recall,
        group_message_recall_handlers,
        GroupMessageRecallEvent
    );
    on_event!(on_new_friend, new_friend_handlers, NewFriendEvent);
    on_event!(on_group_leave, group_leave_handlers, GroupLeaveEvent);
    on_event!(on_friend_poke, friend_poke_handlers, FriendPokeEvent);
    on_event!(
        on_group_name_update,
        group_name_update_handlers,
        GroupNameUpdateEvent
    );
    on_event!(on_delete_friend, delete_friend_handlers, DeleteFriendEvent);
    on_event!(
        on_member_permission_change,
        member_permission_change_handlers,
        MemberPermissionChangeEvent
    );
}

#[allow(clippy::type_complexity)]
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
