use rq_tower::rq::client::event::{FriendMessageEvent, GroupMessageEvent};
use rq_tower::rq::msg::elem::Text;
use rq_tower::rq::msg::MessageChain;

pub async fn print_group(event: GroupMessageEvent) {
    let client = event.client;
    let message = event.message;
    tracing::info!("group_msg({}): {}", message.group_name, message.elements);
    if message.from_uin == 875543533 {
        let mut chain = MessageChain::default();
        chain.push(Text::new("hello".into()));
        client
            .send_group_message(message.group_code, chain)
            .await
            .ok();
    }
}

pub async fn print_friend(FriendMessageEvent { client, message }: FriendMessageEvent) {
    tracing::info!("private_msg({}): {}", message.from_uin, message.elements);
    let mut chain = MessageChain::default();
    chain.push(Text::new("hello".into()));
    client
        .send_friend_message(message.from_uin, chain)
        .await
        .ok();
}
