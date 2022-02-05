use rq_tower::rq::client::event::{GroupMessageEvent, PrivateMessageEvent};
use rq_tower::rq::msg::elem::Text;
use rq_tower::rq::msg::MessageChain;

pub async fn print_group(event: GroupMessageEvent) {
    let group = event.group().await;
    let group_name = group.map(|g| g.info.name.clone()).unwrap_or_default();
    let client = event.client;
    let message = event.message;
    println!("print_group({}): {:?}", group_name, message);
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
    println!("print_private: {:?}", message);
    let mut chain = MessageChain::default();
    chain.push(Text::new("hello".into()));
    client
        .send_private_message(message.from_uin, chain)
        .await
        .ok();
}
