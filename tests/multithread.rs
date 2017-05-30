extern crate simple_nats_client;
#[macro_use]
extern crate log;
use simple_nats_client::nats_client;
use std::time;
use std::sync;
use std::sync::atomic;
use std::thread;
extern crate env_logger;

use std::sync::{Once, ONCE_INIT};

static INIT: Once = ONCE_INIT;

fn setup() {
    INIT.call_once(|| { env_logger::init().unwrap(); })
}
pub fn i32_to_bytes_le(val: i32) -> [u8; 4] {
    return [(val & 0xff) as u8,
            ((val >> 8) & 0xff) as u8,
            ((val >> 16) & 0xff) as u8,
            ((val >> 24) & 0xff) as u8];
}

pub fn bytes_to_i32_le(ar: &[u8]) -> i32 {
    let val = (ar[0] as u32) + ((ar[1] as u32) << 8) + ((ar[2] as u32) << 16) +
              ((ar[3] as u32) << 24);
    return val as i32;
}

#[test]
pub fn test_i32_convert() {
    for i in 0i32..0xff {
        let b = i32_to_bytes_le(i);
        let x = bytes_to_i32_le(&b);
        assert_eq!(i, x);
    }
}

#[test]
pub fn pub_sub_mt_test() {
    setup();
    let queuenum = sync::atomic::AtomicUsize::new(0);
    const MESSAGE_NUM: i32 = 100;
    let subject: String = format!("natsrust.mt.{}",
                                  queuenum.fetch_add(1, atomic::Ordering::Relaxed));
    let (tx, rx) = sync::mpsc::channel::<i32>();
    let consumer_subject = subject.clone();
    let consumer_thread = thread::spawn(move || {
        let timeout = time::Duration::from_secs(3);
        let mut c = nats_client::NatsClient::new_with_option("127.0.0.1", 4222, Some(timeout), None).unwrap();
        let sid = c.subscribe(consumer_subject.as_str(), None).unwrap();
        tx.send(0).unwrap();
        match c.wait_message() {
            Ok(v) => {
                match v {
                    nats_client::NatsResponse::Ok => {}
                    _ => panic!("unexpected subscribe response"),
                }
            }
            Err(e) => panic!("unexpected subscribe error:{:?}", e),
        }
        for i in 0..MESSAGE_NUM {
            debug!("consuming({},{})", consumer_subject, i);
            match c.wait_message() {
                Ok(v) => {
                    match v {
                        nats_client::NatsResponse::Msg(msg) => {
                            let x: i32 = bytes_to_i32_le(msg.data.as_slice());
                            assert_eq!(consumer_subject, msg.subject);
                            assert_eq!(sid, msg.sid);
                            debug!("coming {},{},{},{:?}", i, x, consumer_subject, msg.data);
                            assert_eq!(i, x);
                        }
                        _ => panic!("unexpected response"),
                    }
                }
                Err(e) => panic!("subscribe wait error({}):{:?}", i, e),
            };
        }
    });
    rx.recv().unwrap();
    let producer_subject = subject.clone();
    let producer_thread = thread::spawn(move || {
        let timeout = time::Duration::from_secs(3);
        let mut c = nats_client::NatsClient::new_with_option("127.0.0.1", 4222, Some(timeout), None).unwrap();
        for i in 0..MESSAGE_NUM {
            let dat: [u8; 4] = i32_to_bytes_le(i);
            c.publish(producer_subject.as_str(), None, &dat).unwrap();
            c.wait_message().unwrap();
        }
    });
    producer_thread.join().unwrap();
    consumer_thread.join().unwrap();
}

