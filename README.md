# Simple NATS client for Rust

this is the [NATS](https://nats.io) client library for rust.

# Warning

THIS PROGRAM IS STILL ALPHA VERSION, API MAY BE CHANGED WITHOUT NOTICE.
DO NOT USE FOR PRODUCTION.

# Usage

add following line to Cargo.toml's dependencies entry.(currently, this library is not provided as crate)
```
simple_nats_client = { git = "https://github.com/itn3000/simple_nats_client.git }
```

see [examples](./examples)

# THINGS TO BE PLANNED

* SSL
* PROPER ERROR HANDLING
* AUTO RECONNECT ON UNEXPECTED DISCONNECTION(maybe possible)

# THINGS NOT TO BE PLANNED

* THREAD SAFE
* ASYNCHRONOUS IO