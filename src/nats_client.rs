extern crate log;
extern crate serde_json;

use std::net::TcpStream;
use std::io::Read;
use std::io::Write;
use std::io::Error;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::string::FromUtf8Error;
use std::string::String;
use std::str::FromStr;
use std::fmt;
use std::result;
use std::time;
use std::cmp;
use std::net;

type Result<T> = result::Result<T, NatsError>;
pub type ServerInfo = ::server_info::ServerInfo;
pub type ConnectOption = ::connect_option::ConnectOption;

/// NATS publish/subscribe client
///
/// # Warning
/// 
/// this is not threadsafe because of having stateful stream and internal buffer
pub struct NatsClient {
    tcp_client: TcpStream,
    receive_buffer: Vec<u8>,
    current_sid: AtomicUsize,
    verbose: bool,
    server_info: ServerInfo,
}

/// NATS and another errors
#[derive(Debug)]
pub enum NatsError {
    /// error message notified from server (see: https://nats.io/documentation/internals/nats-protocol/)
    ServerError(NatsServerError),
    /// TCP/IP error,including wait timeout
    ConnectionError(Error, String),
    /// message encoding error(not utf8 data)
    EncodingError(FromUtf8Error),
    /// parse failure from server
    /// first arg is error location, second one is reason.
    MessageParseError(String, String),
    /// protocol parse error
    /// string array separated by ' ' will be passed
    InvalidMessageArgument(Vec<String>),
    /// unknown message format from server
    /// server received message will be passed
    UnknownResponse(String),
    /// Infinite loop detected in wait_message
    WaitInfiniteLoop,
    /// Infinate loop detected in parsing MSG response
    MessageInfiniteLoop,
}

impl fmt::Display for NatsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            &NatsError::ServerError(ref v) => format!("server error:{}", v.error_message),
            &NatsError::ConnectionError(ref e, ref loc) => {
                format!("connection error({}): {}", loc, e)
            }
            &NatsError::EncodingError(ref e) => format!("encoding error:{}", e),
            &NatsError::InvalidMessageArgument(ref args) => {
                format!("invalid message argument:{:?}", args)
            }
            &NatsError::MessageParseError(ref name, ref v) => {
                format!("parse error({}): {}", name, v)
            }
            &NatsError::UnknownResponse(ref v) => format!("unknown message:{}", v),
            &NatsError::WaitInfiniteLoop => format!("infinite wait loop in wait_message"),
            &NatsError::MessageInfiniteLoop => format!("infinite wait loop in parse_message")
        };
        write!(f, "{}", msg)
    }
}

/// NATS normal responses (see: https://nats.io/documentation/internals/nats-protocol/)
pub enum NatsResponse {
    /// message from NATS services
    Msg(NatsMessage),
    /// come when verbose setting is true
    Ok,
    /// keepalive message
    Ping,
    /// keepalive message response
    Pong,
    /// server information
    Info(ServerInfo),
}

#[derive(Debug)]
pub struct NatsServerError {
    pub error_message: String,
}

impl fmt::Display for NatsServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error_message)
    }
}

#[derive(Debug)]
pub struct NatsMessage {
    pub sid: u64,
    pub data: Vec<u8>,
    pub subject: String,
    pub reply: Option<String>,
}

impl NatsClient {
    /// Constructs a new NatsClient
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// extern crate simple_nats_client;
    /// let c = simple_nats_client::nats_client::NatsClient::new("127.0.0.1", 4222).unwrap();
    /// ```
    pub fn new(host: &str, port: i32) -> Result<Self> {
        Self::new_internal(host, port, None, None)
    }

    /// Constructs a new NatsClient with CONNECT call
    ///
    /// # Examples
    ///
    /// ```
    /// extern crate simple_nats_client;
    /// use std::time::Duration;
    /// let c = simple_nats_client::nats_client::NatsClient::new_with_option("127.0.0.1", 4222, Some(Duration::from_secs(60)), Some(&simple_nats_client::nats_client::ConnectOption::new()));
    /// ```
    pub fn new_with_option(host: &str,
                           port: i32,
                           read_timeout: Option<time::Duration>,
                           connect_option: Option<&ConnectOption>)
                           -> Result<Self> {
        Self::new_internal(host, port, read_timeout, connect_option)
    }
    fn new_internal(host: &str,
                    port: i32,
                    read_timeout: Option<time::Duration>,
                    opt: Option<&ConnectOption>)
                    -> Result<Self> {
        let mut client = match TcpStream::connect(format!("{}:{}", host, port)) {
            Ok(v) => v,
            Err(e) => return Err(NatsError::ConnectionError(e, "NatsClient::new".to_owned())),
        };
        match read_timeout {
            Some(t) => Self::set_read_timeout_internal(&mut client, Some(t))?,
            None => {}
        };
        let mut buf = Vec::new();
        buf.reserve(1024);
        let mut infobuf = [0u8; 1024];
        let bytesread = Self::read_request(&mut client,
                                           &mut infobuf,
                                           "NatsClient::new::get_server_info")?;
        let parsed = match String::from_utf8(infobuf[0..bytesread].to_vec()) {
            Ok(v) => v,
            Err(e) => return Err(NatsError::EncodingError(e)),
        };
        let args: Vec<&str> = parsed.split(" ").collect();
        let server_info: ServerInfo = match NatsClient::parse_server_info(&args[1..]) {
            Ok(v) => {
                match v {
                    NatsResponse::Info(v) => v,
                    _ => return Err(NatsError::UnknownResponse(parsed.clone())),
                }
            }
            Err(e) => return Err(e),
        };
        debug!("{:?}", server_info);
        let mut ret = NatsClient {
            tcp_client: client,
            receive_buffer: buf,
            current_sid: AtomicUsize::new(0),
            verbose: match &opt {
                &Some(ref v) => v.verbose,
                &None => true,
            },
            server_info: server_info,
        };
        match opt {
            Some(opt) => ret.send_connect_option(&opt)?,
            None => {}
        }
        Ok(ret)
    }
    fn set_read_timeout_internal(client: &mut TcpStream, t: Option<time::Duration>) -> Result<()> {
        match client.set_read_timeout(t) {
            Ok(_) => Ok(()),
            Err(e) => {
                return Err(NatsError::ConnectionError(e,
                                                      "NatsClient::set_read_timeout_internal"
                                                          .to_owned()))
            }
        }
    }
    fn send_connect_option(&mut self, opt: &ConnectOption) -> Result<()> {
        let connectstr = match serde_json::to_string(opt) {
            Ok(v) => v,
            Err(e) => {
                return Err(NatsError::MessageParseError(format!("json serialization error"),
                                                        format!("{:?}",e)))
            }
        };
        Self::write_request(&mut self.tcp_client,
                            format!("CONNECT {}\r\n", connectstr).as_bytes(),
                            "NatsClient::send_connect_option")?;
        if opt.verbose {
            let mut buf = [0u8; 32];
            Self::read_request(&mut self.tcp_client,
                               &mut buf,
                               "NatsClient::send_connect_option")?;
        }
        // Self::write_request(&self.tcp_client, )
        Ok(())
    }
    /// Publish message to specified subject
    ///
    /// # Examples
    /// 
    /// ```
    /// extern crate simple_nats_client;
    /// let mut c = simple_nats_client::nats_client::NatsClient::new("127.0.0.1",4222).unwrap();
    /// c.publish("subject", None, &[0u8;4]).unwrap();
    /// ```
    pub fn publish(&mut self, subject: &str, reply_to: Option<&str>, data: &[u8]) -> Result<u64> {
        let datastr: String = match reply_to {
            Some(v) => format!("PUB {} {} {}\r\n", subject, v, data.len()),
            None => format!("PUB {} {}\r\n", subject, data.len()),
        };
        // debug!("publishing string:{}", datastr);
        Self::write_request(&mut self.tcp_client,
                            datastr.as_bytes(),
                            "NatsClient::publish")?;
        Self::write_request(&mut self.tcp_client, data, "NatsClient::publish")?;
        let mut crlf_bytes: [u8; 2] = [0x0d, 0x0a];
        Self::write_request(&mut self.tcp_client, &mut crlf_bytes, "NatsClient::publish")?;
        if self.verbose {
            self.consume_verbose_response()?;
            // debug!("publish({}): consume verbose message done", subject);
        }
        Ok(0)
    }
    /// Subscribe specified subject, returns Subscription ID
    /// 
    /// if you want to get events, do wait_message.
    /// sid is unique in instance.
    /// It means instance's sid may be equal to another instance sid
    /// 
    /// # Examples
    ///
    /// ```
    /// extern crate simple_nats_client;
    /// let mut c = simple_nats_client::nats_client::NatsClient::new("127.0.0.1", 4222).unwrap();
    /// let sid = c.subscribe("subject", None).unwrap();
    /// let sid2 = c.subscribe("subject2", Some("qname")).unwrap();
    /// ```
    pub fn subscribe(&mut self, subject: &str, queue: Option<&str>) -> Result<u64> {
        let sid = self.current_sid.fetch_add(1, Ordering::Relaxed);
        let datastr: String = match queue {
            Some(v) => format!("SUB {} {} {}\r\n", subject, v, sid),
            None => format!("SUB {} {}\r\n", subject, sid),
        };
        debug!("subscribing string:{}", datastr);
        Self::write_request(&mut self.tcp_client, &mut datastr.as_bytes(), "subscribe")?;
        if self.verbose {
            self.consume_verbose_response()?;
            debug!("subscribe({}):consume verbose message done", subject);
        }
        Ok(sid as u64)
    }
    /// wait server response
    ///
    /// # Examples
    /// 
    /// ```
    /// extern crate simple_nats_client;
    /// let mut c = simple_nats_client::nats_client::NatsClient::new("127.0.0.1", 4222).unwrap();
    /// let sid = c.subscribe("subject", None).unwrap();
    /// c.publish("subject", None, &[1u8;4]).unwrap();
    /// let response = c.wait_message().unwrap();
    /// match response {
    ///   simple_nats_client::nats_client::NatsResponse::Msg(v) => println!("{:?}", v),
    ///   _ => println!("another message")
    /// };
    /// ```
    pub fn wait_message(&mut self) -> Result<NatsResponse> {
        for i in 0..1000 {
            if let Some(crlf_index) = Self::find_crlf(self.receive_buffer.as_slice()) {
                let header_string: String =
                    match String::from_utf8(self.receive_buffer[0..crlf_index].to_vec()) {
                        Ok(v) => v,
                        Err(e) => return Err(NatsError::EncodingError(e)),
                    };
                let headers: Vec<&str> = header_string.split(" ").collect();
                // remove consumed data
                self.receive_buffer = self.receive_buffer[crlf_index + 2..].to_vec();
                return match headers[0] {
                           "-ERR" => {
                               Err(NatsError::ServerError(NatsServerError {
                                                              error_message: headers[1..].join(" "),
                                                          }))
                           }
                           "+OK" => Ok(NatsResponse::Ok),
                           "MSG" => self.parse_message(&headers[1..]),
                           "PING" => Ok(NatsResponse::Ping),
                           "PONG" => Ok(NatsResponse::Pong),
                           "INFO" => NatsClient::parse_server_info(&headers[1..]),
                           _ => Err(NatsError::UnknownResponse(headers.join(""))),
                       };
            } else {
                let mut buf = [0u8; 512];
                debug!("reading nats message from server,{}", i);
                let bytesread =
                    Self::read_request(&mut self.tcp_client, &mut buf, "NatsClient::wait_message")?;
                self.receive_buffer
                    .append(&mut buf[0..bytesread].to_vec());
            }
        }
        Err(NatsError::WaitInfiniteLoop)
    }
    fn parse_message(&mut self, args: &[&str]) -> Result<NatsResponse> {
        debug!("parsing message:{:?}", args);
        let (subject, sidstr, reply, msgsizestr) = match args.len() {
            3 => (args[0], args[1], None as Option<&str>, args[2]),
            4 => (args[0], args[1], Some(args[2]), args[3]),
            _ => {
                return Err(NatsError::InvalidMessageArgument(args.iter()
                                                                 .map(|&x| String::from(x))
                                                                 .collect()))
            }
        };
        let sid = match u64::from_str(sidstr) {
            Ok(v) => v,
            Err(e) => {
                return Err(NatsError::MessageParseError(format!("sid:{}", e), sidstr.to_owned()))
            }
        };
        let msgsize = match i64::from_str(msgsizestr) {
            Ok(v) => v,
            Err(e) => {
                return Err(NatsError::MessageParseError(format!("msgsize:{}", e),
                                                        msgsizestr.to_owned()))
            }
        };
        debug!("subject = {}, sid = {}, msgsize = {}, reply = {:?}, current buffer length = {}",
               subject,
               sid,
               msgsize,
               reply,
               self.receive_buffer.len());
        let copylen = cmp::min((msgsize + 2) as usize, self.receive_buffer.len());
        let mut data: Vec<u8> = vec![0u8;(msgsize + 2) as usize];
        data[0..copylen].copy_from_slice(&self.receive_buffer[0..copylen]);
        let remaining_length = self.receive_buffer.len() - copylen;
        self.receive_buffer[0..remaining_length].copy_from_slice(&self.receive_buffer[copylen..]);
        self.receive_buffer.truncate(remaining_length);
        let mut total_bytes_read: usize = copylen;
        for i in 0..(msgsize + 1) * 100 {
            debug!("consuming buffer:{},{},{}", i, total_bytes_read, msgsize);
            // msgsize + CRLF
            if total_bytes_read >= ((msgsize + 2) as usize) {
                // remove CRLF
                data.truncate(msgsize as usize);
                return Ok(NatsResponse::Msg(NatsMessage {
                                                subject: subject.to_owned(),
                                                sid: sid,
                                                data: data,
                                                reply: match reply {
                                                    Some(v) => Some(v.to_owned()),
                                                    None => None,
                                                },
                                            }));
            } else {
                debug!("recv from remote");
                let bytesread = Self::read_request(&mut self.tcp_client,
                                                   &mut data[total_bytes_read..],
                                                   "NatsClient::parse_message")?;
                total_bytes_read += bytesread;
                // data.append(&mut buf[0..bytesread]);
                // self.receive_buffer
                //     .append(&mut buf[0..bytesread].to_vec());
            }
        }
        Err(NatsError::MessageInfiniteLoop)
    }
    fn parse_server_info(args: &[&str]) -> Result<NatsResponse> {
        let svr_info: ServerInfo = match serde_json::from_str(&args.join(" ")) {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        };
        Ok(NatsResponse::Info(svr_info))
    }
    /// unsubscribe specified subscription ID.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// extern crate simple_nats_client;
    /// use std::time;
    /// let mut c = simple_nats_client::nats_client::NatsClient::new("127.0.0.1", 4222).unwrap();
    /// let sid = c.subscribe("subject", None).unwrap();
    /// c.set_read_timeout(Some(time::Duration::from_secs(1))).unwrap();
    /// c.unsubscribe(sid).unwrap();
    /// for i in 0..100 {
    ///     // this wait_message should be timeouted after consuming "+OK" messages
    ///     let msg = c.wait_message().unwrap();
    ///     match msg {
    ///         simple_nats_client::nats_client::NatsResponse::Msg(msg) => println!("{:?}", msg),
    ///         _ => println!("another response")
    ///     }
    /// }
    /// ```
    pub fn unsubscribe(&mut self, sid: u64) -> Result<()> {
        self.unsubscribe_internal(sid, None)
    }
    /// unsubscribe specified subscription ID after receiving specified number of message.
    pub fn unsubscribe_after(&mut self, sid: u64, unsubscribe_after: i32) -> Result<()> {
        self.unsubscribe_internal(sid, Some(unsubscribe_after))
    }
    fn unsubscribe_internal(&mut self, sid: u64, unsubscribe_after: Option<i32>) -> Result<()> {
        let datastr = match unsubscribe_after {
            Some(v) => format!("UNSUB {} {}\r\n", sid, v),
            None => format!("UNSUB {}\r\n", sid),
        };
        Self::write_request(&mut self.tcp_client,
                            datastr.as_bytes(),
                            "NatsClient::unsubscribe")?;
        Ok(())
    }
    /// set read timeout for wait_message
    pub fn set_read_timeout(&mut self, timeout: Option<time::Duration>) -> Result<()> {
        Self::set_read_timeout_internal(&mut self.tcp_client, timeout)
    }
    fn write_request(c: &mut Write, data: &[u8], from: &str) -> Result<usize> {
        match c.write(data) {
            Ok(v) => Ok(v),
            Err(e) => Err(NatsError::ConnectionError(e, from.to_owned())),
        }
    }
    fn read_request(c: &mut Read, buf: &mut [u8], from: &str) -> Result<usize> {
        match c.read(buf) {
            Ok(v) => Ok(v),
            Err(e) => Err(NatsError::ConnectionError(e, from.to_owned())),
        }
    }
    fn find_crlf(dat: &[u8]) -> Option<usize> {
        // debug!("finding crlf(buflen={:?})", dat.len());
        if dat.len() < 2 {
            debug!("buffer to short");
            return None;
        }
        for i in 0..dat.len() - 1 {
            if dat[i] == 0x0d && dat[i + 1] == 0x0a {
                // debug!("found crlf:{}", i);
                return Some(i);
            }
        }
        debug!("no crlf found");
        return None;
    }
    fn consume_verbose_response(&mut self) -> Result<()> {
        // debug!("receiving verbose message");
        let mut buf = [0u8; 512];
        let bytesread = match self.tcp_client.read(&mut buf) {
            Ok(v) => v,
            Err(e) => {
                return Err(NatsError::ConnectionError(e, "NatsClient::publish::read_ok".to_owned()))
            }
        };
        self.receive_buffer
            .append(&mut buf[0..bytesread].to_vec());
        // debug!("receiving verbose message done:{}", bytesread);
        Ok(())
    }
    /// getter for ServerInfo coming from server
    pub fn get_server_info(&self) -> &ServerInfo {
        &self.server_info
    }
}

impl Drop for NatsClient {
    fn drop(&mut self) {
        self.tcp_client
            .shutdown(net::Shutdown::Both)
            .unwrap_or_default();
    }
}

