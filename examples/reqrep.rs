extern crate simple_nats_client;

use simple_nats_client::nats_client::*;

fn main() {
    let opt = ConnectOption::new_with_param("", "", false, "");
    let mut c = NatsClient::new_with_option("127.0.0.1", 4222, None, Some(&opt)).unwrap();
    const SUBJECT: &str = "natstest.subject";
    const REPLY_SUBJECT: &str = "natstest.replyto";
    let sid = c.subscribe(SUBJECT, None).unwrap();
    c.publish(SUBJECT, Some(REPLY_SUBJECT), &[0u8, 1u8, 2u8, 3u8])
        .unwrap();
    // before you send publish , you must create inbox subscription
    let replysid = c.subscribe(REPLY_SUBJECT, None).unwrap();
    // auto deletion after receiving reply.
    c.unsubscribe_after(replysid, 1).unwrap();
    let msg = c.wait_message().unwrap();
    match msg {
        NatsResponse::Msg(v) => {
            println!("msg: {:?}", v);
            if let Some(replyto) = v.reply {
                c.publish(replyto.as_str(), None, &[3u8, 2u8, 1u8, 0u8]).unwrap();
            }
        }
        _ => panic!("unexpected message"),
    };
    let reply_response = c.wait_message().unwrap();
    match reply_response {
        NatsResponse::Msg(v) => println!("msg: {:?}", v),
        _ => println!("another message"),
    };
    c.unsubscribe(sid).unwrap();
}

