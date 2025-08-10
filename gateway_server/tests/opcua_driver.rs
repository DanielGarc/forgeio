use gateway_server::drivers::opcua::OpcUaDriver;
use gateway_server::drivers::traits::{OpcDriver, OpcDriverConfig};
use opcua::server::address_space::Variable;
use opcua::server::diagnostics::NamespaceMetadata;
use opcua::server::node_manager::memory::{simple_node_manager, SimpleNodeManager};
use opcua::server::{ServerBuilder, ServerHandle};
use opcua::types::NodeId;
use tokio::time::{sleep, Duration};

struct DummyServer {
    handle: ServerHandle,
    _task: tokio::task::JoinHandle<()>,
}

impl DummyServer {
    async fn start() -> Self {
        let namespace_uri = "http://forgeio/dummy/";
        let (server, handle) = ServerBuilder::new_anonymous("Dummy OPC UA Server")
            .host("127.0.0.1")
            .port(4840)
            .with_node_manager(simple_node_manager(
                NamespaceMetadata {
                    namespace_uri: namespace_uri.to_string(),
                    ..Default::default()
                },
                "dummy",
            ))
            .build()
            .unwrap();

        let node_manager = handle
            .node_managers()
            .get_of_type::<SimpleNodeManager>()
            .unwrap();
        let ns = handle.get_namespace_index(namespace_uri).unwrap();
        {
            let mut space = node_manager.address_space().write();
            let _ = space.add_variables(
                vec![
                    Variable::new(
                        &NodeId::new(ns, "Temperature"),
                        "Temperature",
                        "Temperature",
                        20f64,
                    ),
                    Variable::new(&NodeId::new(ns, "Pressure"), "Pressure", "Pressure", 1f64),
                    Variable::new(&NodeId::new(ns, "Counter"), "Counter", "Counter", 0i32),
                ],
                &NodeId::objects_folder_id(),
            );
        }

        let task = tokio::spawn(async move {
            server.run().await.unwrap();
        });
        sleep(Duration::from_secs(1)).await;
        DummyServer {
            handle,
            _task: task,
        }
    }
}

impl Drop for DummyServer {
    fn drop(&mut self) {
        self.handle.cancel();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn browse_tags_from_dummy_server() {
    let _ = tracing_subscriber::fmt::try_init();
    let _server = DummyServer::start().await;
    let config = OpcDriverConfig {
        id: "srv".into(),
        name: "srv".into(),
        address: "opc.tcp://127.0.0.1:4840/".into(),
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

    let tags = driver.browse_node("ns=0;i=85").await.unwrap();
    assert!(tags.contains(&"Temperature".to_string()));
    assert!(tags.contains(&"Pressure".to_string()));
    assert!(tags.contains(&"Counter".to_string()));

    driver.disconnect().await.unwrap();
}
