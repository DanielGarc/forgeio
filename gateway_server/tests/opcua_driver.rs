use gateway_server::drivers::opcua::OpcUaDriver;
use gateway_server::drivers::traits::{DeviceDriver, DriverConfig, TagRequest};
use std::process::{Child, Command};
use std::thread::sleep as std_sleep;
use std::time::Duration;
use tokio::runtime::Runtime;

struct ServerHandle(Child);

impl Drop for ServerHandle {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

fn start_server() -> ServerHandle {
    let _ = Command::new("python")
        .args(["-m", "pip", "install", "--quiet", "asyncua"])
        .status();
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // move out of gateway_server
    path.push("examples/dummy_opcua_server.py");
    let child = Command::new("python")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start server");
    ServerHandle(child)
}

#[test]
#[ignore]
fn read_tag_from_dummy_server() {
    let _server = start_server();
    std_sleep(Duration::from_secs(2));

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
    };
    let mut driver = OpcUaDriver::new(config).unwrap();
    if let Err(e) = driver.connect() {
        eprintln!("OPC UA connect failed: {:?}, skipping test", e);
        return;
    }

    let requests = vec![
        TagRequest {
            address: "ns=2;s=Temperature".into(),
        },
        TagRequest {
            address: "ns=2;s=Pressure".into(),
        },
        TagRequest {
            address: "ns=2;s=Counter".into(),
        },
    ];
    let rt = Runtime::new().unwrap();
    let result = rt.block_on(driver.read_tags(&requests)).unwrap();
    assert!(result.contains_key("ns=2;s=Temperature"));
    assert!(result.contains_key("ns=2;s=Pressure"));
    assert!(result.contains_key("ns=2;s=Counter"));


    driver.disconnect().unwrap();
}
