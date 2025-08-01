use gateway_server::drivers::opcua::OpcUaDriver;
use gateway_server::drivers::traits::{DeviceDriver, DriverConfig, TagRequest};
use std::process::{Command, Child};
use std::time::Duration;
use std::thread::sleep as std_sleep;
use tokio::runtime::Runtime;

fn start_server() -> Child {
    let _ = Command::new("python")
        .args(["-m", "pip", "install", "--quiet", "asyncua"])
        .status();
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // move out of gateway_server
    path.push("examples/dummy_opcua_server.py");
    Command::new("python")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start server")
}

#[test]
#[ignore]
fn read_tag_from_dummy_server() {
    let mut server = start_server();
    std_sleep(Duration::from_secs(2));

    let config = DriverConfig {
        id: "srv".into(),
        name: "srv".into(),
        address: "opc.tcp://localhost:4840/freeopcua/server/".into(),
        scan_rate_ms: 1000,
    };
    let mut driver = OpcUaDriver::new(config).unwrap();
    driver.connect().unwrap();

    let requests = vec![TagRequest { address: "ns=2;s=Counter".into() }];
    let rt = Runtime::new().unwrap();
    let result = rt.block_on(driver.read_tags(&requests)).unwrap();
    assert!(result.contains_key("ns=2;s=Counter"));

    driver.disconnect().unwrap();
    let _ = server.kill();
}
