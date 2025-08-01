use gateway_server::tags::engine::TagEngine;
use gateway_server::tags::structures::{Tag, TagValue, ValueVariant, TagMetadata, Quality};

fn sample_tag(path: &str, driver_id: &str, address: &str) -> Tag {
    Tag {
        path: path.to_string(),
        value: TagValue::new(ValueVariant::Int(0), Quality::Good),
        driver_id: driver_id.to_string(),
        driver_address: address.to_string(),
        poll_rate_ms: 1000,
        metadata: TagMetadata::default(),
    }
}

#[test]
fn register_and_read_tag() {
    let engine = TagEngine::new();
    let tag = sample_tag("Device/Tag1", "drv1", "addr1");
    engine.register_tag(tag.clone());

    let read = engine.read_tag("Device/Tag1").expect("tag should exist");
    assert_eq!(read, tag.value);
}

#[test]
fn update_tag_value() {
    let engine = TagEngine::new();
    let tag = sample_tag("Device/Tag2", "drv1", "addr2");
    engine.register_tag(tag.clone());

    let new_value = TagValue::new(ValueVariant::Int(42), Quality::Good);
    let updated = engine.update_tag_value(&tag.path, new_value.clone());
    assert!(updated);
    let read = engine.read_tag(&tag.path).unwrap();
    assert_eq!(read, new_value);
}

#[test]
fn list_paths_and_find_by_address() {
    let engine = TagEngine::new();
    let tag1 = sample_tag("Device/TagA", "drv1", "a1");
    let tag2 = sample_tag("Device/TagB", "drv1", "a2");
    engine.register_tag(tag1.clone());
    engine.register_tag(tag2.clone());

    let mut paths = engine.get_all_tag_paths();
    paths.sort();
    assert_eq!(paths, vec![tag1.path.clone(), tag2.path.clone()]);

    assert_eq!(engine.find_path_by_address("drv1", "a1"), Some(tag1.path));
    assert_eq!(engine.find_path_by_address("drv1", "a2"), Some(tag2.path));
}

#[test]
fn get_tag_details_and_all_tags() {
    let engine = TagEngine::new();
    let tag = sample_tag("Device/TagC", "drv2", "addrC");
    engine.register_tag(tag.clone());

    let details = engine.get_tag_details(&tag.path).expect("details");
    assert_eq!(details.driver_id, tag.driver_id);

    let all = futures::executor::block_on(engine.get_all_tags());
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].path, tag.path);
}
