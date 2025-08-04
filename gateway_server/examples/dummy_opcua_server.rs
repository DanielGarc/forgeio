use opcua::server::address_space::Variable;
use opcua::server::diagnostics::NamespaceMetadata;
use opcua::server::node_manager::memory::{simple_node_manager, SimpleNodeManager};
use opcua::server::ServerBuilder;
use opcua::types::NodeId;

#[tokio::main]
async fn main() {
    let namespace_uri = "http://forgeio/dummy/";
    let (server, handle) = ServerBuilder::new_anonymous("Dummy OPC UA Server")
        .host("0.0.0.0")
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
    let address_space = node_manager.address_space();
    {
        let mut space = address_space.write();
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

    server.run().await.unwrap();
}
