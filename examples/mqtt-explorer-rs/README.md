# paho-mqtt example in Rust

To enable logging in stdout (must init env-logger in the code i.e. `env_logger::init()`):
```
RUST_LOG="paho_mqtt=info,paho_mqtt_c=debug"  cargo run
```
