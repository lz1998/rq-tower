use rq_tower::rq::client::event::{GroupMessageEvent, PrivateMessageEvent};
use rq_tower::rq::msg::elem::Text;
use rq_tower::rq::msg::MessageChain;

pub async fn print_group(event: GroupMessageEvent) {
    let group = event.group().await;
    let group_name = group.map(|g| g.info.name.clone()).unwrap_or_default();
    let client = event.client;
    let message = event.message;
    tracing::info!("group_msg({}): {}", group_name, message.elements);
    if message.from_uin == 875543533 {
        let mut chain = MessageChain::default();
        chain.push(Text::new("hello".into()));
        client
            .send_group_message(message.group_code, chain)
            .await
            .ok();
    }
}

pub async fn print_private(PrivateMessageEvent { client, message }: PrivateMessageEvent) {
    tracing::info!("private_msg({}): {}", message.from_uin, message.elements);
    let mut chain = MessageChain::default();
    chain.push(Text::new("hello".into()));
    client
        .send_private_message(message.from_uin, chain)
        .await
        .ok();
}
