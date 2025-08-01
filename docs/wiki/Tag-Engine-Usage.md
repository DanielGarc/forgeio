# Tag Engine Usage

This page demonstrates how to use the `TagEngine` found in the `gateway_server` crate. The engine manages tag definitions and values in-memory and provides thread-safe access for concurrent applications.

## Creating a `TagEngine`

```rust
use gateway_server::tags::engine::TagEngine;

let engine = TagEngine::new();
```

## Registering Tags

Before a tag can be read or updated, it must be registered with the engine:

```rust
use gateway_server::tags::structures::{Tag, TagValue, ValueVariant, TagMetadata, Quality};

let tag = Tag {
    path: "Device/Temperature".into(),
    value: TagValue::new(ValueVariant::Int(0), Quality::Good),
    driver_id: "device1".into(),
    driver_address: "ns=1;s=Temp".into(),
    poll_rate_ms: 1000,
    metadata: TagMetadata::default(),
};

engine.register_tag(tag);
```

## Reading a Tag

```rust
if let Some(value) = engine.read_tag("Device/Temperature") {
    println!("Current value: {:?}", value);
}
```

## Updating a Tag's Value

```rust
let new_value = TagValue::new(ValueVariant::Int(25), Quality::Good);
let success = engine.update_tag_value("Device/Temperature", new_value);
assert!(success);
```

## Browsing Tags

```rust
// List all tag paths
let mut paths = engine.get_all_tag_paths();
paths.sort();

// Find a tag path by driver id and address
if let Some(path) = engine.find_path_by_address("device1", "ns=1;s=Temp") {
    println!("Tag path is {}", path);
}
```

## Getting Detailed Information

```rust
if let Some(tag) = engine.get_tag_details("Device/Temperature") {
    println!("Driver: {}", tag.driver_id);
}

// Retrieve all tag structures asynchronously
let all_tags = futures::executor::block_on(engine.get_all_tags());
println!("Loaded {} tags", all_tags.len());
```

---

These snippets can be combined in your own application to manage tags in a thread-safe manner. The Tag Engine is designed to scale to thousands or millions of tags depending on your use case.
