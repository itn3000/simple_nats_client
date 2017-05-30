#![feature(test)]

extern crate test;

extern crate simple_nats_client;

use simple_nats_client::nats_client;
use simple_nats_client::connect_option;
use std::time;
extern crate serde_json;
extern crate env_logger;
use std::sync;
use std::thread;
use std::sync::atomic;

fn i32_to_bytes_le(val: i32) -> [u8; 4] {
    return [(val & 0xff) as u8,
            ((val >> 8) & 0xff) as u8,
            ((val >> 16) & 0xff) as u8,
            ((val >> 24) & 0xff) as u8];
}

#[bench]
pub fn pub_sub_bench(b: &mut test::Bencher) {
    let queuenum = sync::atomic::AtomicUsize::new(0);
    b.iter(|| {
        const MESSAGE_NUM: i32 = 1000;
        let subject: String = format!("natsrust.pubsubbench.{}",
                                      queuenum.fetch_add(1, atomic::Ordering::Relaxed));
        let (tx, rx) = sync::mpsc::channel::<i32>();
        let consumer_subject = subject.clone();
        let consumer_thread = thread::spawn(move || {
            let timeout = time::Duration::from_secs(3);
            let opt = connect_option::ConnectOption::new_with_param("", "", true, "pubsubbench-subscriber");
            let mut c = nats_client::NatsClient::new_with_option("127.0.0.1", 4222, Some(timeout), Some(&opt)).unwrap();
            c.subscribe(consumer_subject.as_str(), None).unwrap();
            tx.send(0).unwrap();
            match c.wait_message(None) {
                Ok(v) => {
                    match v {
                        nats_client::NatsResponse::Ok => {}
                        _ => panic!("unexpected subscribe response"),
                    }
                }
                Err(e) => panic!("unexpected subscribe error:{:?}", e),
            }
            for i in 0..MESSAGE_NUM {
                match c.wait_message() {
                    Ok(v) => {
                        match v {
                            nats_client::NatsResponse::Msg(_) => {}
                            _ => panic!("unexpected response({})", i),
                        }
                    },
                    Err(e) => panic!("subscribe wait error({}):{:?}", i, e)
                };
            }
        });
        rx.recv().unwrap();
        let producer_subject = subject.clone();
        let producer_thread = thread::spawn(move || {
            let timeout = time::Duration::from_secs(3);
            let opt = connect_option::ConnectOption::new_with_param("", "", false, "pubsubbench-publisher");
            let mut c = nats_client::NatsClient::new_with_option("127.0.0.1", 4222, Some(timeout), Some(&opt)).unwrap();
            for i in 0..MESSAGE_NUM {
                c.publish(producer_subject.as_str(), None, &i32_to_bytes_le(i)).unwrap();
            }
        });
        producer_thread.join().unwrap();
        consumer_thread.join().unwrap();
    })

}

