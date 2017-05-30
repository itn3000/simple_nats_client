extern crate simple_nats_client;

use simple_nats_client::nats_client::*;

fn main() {
    let opt = ConnectOption::new_with_param("","",false,"");
    let mut c = NatsClient::new_with_option("127.0.0.1", 4222, None, Some(&opt)).unwrap();
    const SUBJECT: &str = "subject";
    let sid = c.subscribe(SUBJECT, None).unwrap();
    c.publish(SUBJECT, None, &[0u8, 1u8, 2u8, 3u8]).unwrap();
    for i in 0..3 {
        let msg = c.wait_message().unwrap();
        println!("receiving msg:{}", i);
        match msg {
            NatsResponse::Ok => println!("ok:{}", i),
            NatsResponse::Msg(v) => {
                println!("msg({}): {:?}", i, v);
                break;
            }
            NatsResponse::Ping => println!("ping"),
            NatsResponse::Pong => println!("pong"),
            NatsResponse::Info(v) => println!("info: {:?}", v),

        };
    }
    c.unsubscribe(sid).unwrap();
}

