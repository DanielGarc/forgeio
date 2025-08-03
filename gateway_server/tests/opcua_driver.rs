use gateway_server::drivers::opcua::OpcUaDriver;
use gateway_server::drivers::traits::{DeviceDriver, DriverConfig};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::{path::PathBuf, thread::sleep};

struct DummyServer(Child);

impl DummyServer {
    fn start() -> Self {
        let _ = Command::new("python")
            .args(["-m", "pip", "install", "--quiet", "asyncua==0.9.92"])
            .status();
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.push("examples/dummy_opcua_server.py");
        let mut child = Command::new("python")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start server");
        // give the server a moment to initialize
        sleep(Duration::from_secs(2));
        if child.try_wait().expect("wait failed").is_some() {
            panic!("server exited immediately");
        }
        DummyServer(child)
    }
}

impl Drop for DummyServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn browse_tags_from_dummy_server() {
    let _ = tracing_subscriber::fmt::try_init();
    let _server = DummyServer::start();
    let config = DriverConfig {
        id: "srv".into(),
        name: "srv".into(),
        address: "opc.tcp://127.0.0.1:4840/freeopcua/server/".into(),
        scan_rate_ms: 1000,
        application_name: Some("TestClient".into()),
        application_uri: None,
        session_name: Some("TestSession".into()),
        max_message_size: None,
        max_chunk_count: None,
        connect_retry_attempts: Some(10),
        connect_retry_delay_ms: Some(200),
        connect_retry_backoff: Some(1.5),
        connect_timeout_ms: Some(1000),
    };
    let driver = OpcUaDriver::new(config).unwrap();
    driver.connect().await.unwrap();
    driver.check_status().await.unwrap();

    let tags = driver.browse_node("ns=0;i=85").unwrap();
    assert!(tags.contains(&"Temperature".to_string()));
    assert!(tags.contains(&"Pressure".to_string()));
    assert!(tags.contains(&"Counter".to_string()));

    driver.disconnect().await.unwrap();
}
