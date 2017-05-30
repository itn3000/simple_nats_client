extern crate simple_nats_client;

use simple_nats_client::server_info;
use simple_nats_client::nats_client;
use simple_nats_client::connect_option;
use std::time;
extern crate serde_json;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::sync::{Once, ONCE_INIT};

static INIT: Once = ONCE_INIT;

fn setup() {
    INIT.call_once(|| { env_logger::init().unwrap(); })
}
// use test_setup;
#[test]
pub fn connect_test() {
    setup();
    let mut opt = connect_option::ConnectOption::new_with_param("", "", false, "testclient");
    opt.verbose = false;
    match nats_client::NatsClient::new_with_option("127.0.0.1", 4222, Some(time::Duration::from_secs(3)), Some(&opt)) {
        Ok(c) => {
            assert_eq!(4222, c.get_server_info().port);
        },
        Err(e) => panic!("failed to connect:{}", e),
    }
}
#[test]
pub fn publish_test() {
    setup();
    let mut c = nats_client::NatsClient::new("127.0.0.1", 4222)
        .unwrap();
    let subject = "natsrust.pub";
    let dat = [0u8; 8];
    let replyto = None as Option<&str>;
    let qname = None as Option<&str>;
    debug!("publish_test::begin publish");
    c.publish(subject, replyto, &dat).unwrap();
    debug!("publish_test::publishing done, do subscribing");
    c.subscribe(subject, qname).unwrap();
    debug!("publish_test::subscribing done");
}
#[test]
pub fn pub_sub_test() {
    setup();
    let opt = connect_option::ConnectOption::new_with_param("", "", false, "testclient");
    let mut c = nats_client::NatsClient::new_with_option("127.0.0.1", 4222, Some(time::Duration::from_secs(3)), Some(&opt))
        .unwrap();
    let subject = "natsrust.pubsub";
    let qname = None as Option<&str>;
    let sid = c.subscribe(subject, qname).unwrap();
    for loop_count in 0..100 {
        let replyto = Some("replyto");
        let mut dat = [0u8; 8];
        dat[0] = (loop_count % 0xff) as u8;
        c.publish(subject, replyto, &dat).unwrap();
        for i in 0..100 {
            debug!("loop({},{})", loop_count, i);
            let msg = c.wait_message().unwrap();
            match msg {
                nats_client::NatsResponse::Msg(v) => {
                    assert_eq!(subject, v.subject);
                    assert_eq!(dat.len(), v.data.len());
                    for j in 0..dat.len() {
                        assert_eq!(dat[j], v.data[j]);
                    }
                    assert_eq!(Some("replyto").unwrap(), v.reply.unwrap());
                    assert_eq!(sid, v.sid);
                    break;
                }
                nats_client::NatsResponse::Ok => {
                }
                nats_client::NatsResponse::Ping => panic!("unexpected ping"),
                nats_client::NatsResponse::Pong => panic!("unexpected pong"),
                nats_client::NatsResponse::Info(v) => {
                    debug!("{:?}", v);
                }
            }
        }
    }
}
#[test]
pub fn serialize_server_info() {
    setup();
    let mut svr_info = server_info::ServerInfo::default();
    svr_info.go = "hogehoge".to_owned();
    let jsonstr = serde_json::to_string(&svr_info).unwrap();
    let x: server_info::ServerInfo = serde_json::from_str(jsonstr.as_str()).unwrap();
    assert!(x.go == svr_info.go);
}

