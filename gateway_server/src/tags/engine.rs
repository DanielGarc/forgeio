use crate::tags::structures::{Tag, TagValue, Quality};
use dashmap::DashMap; // Using DashMap for concurrent R/W access
use std::sync::Arc;

/// Manages the state of all tags in the system.
/// Uses DashMap for thread-safe access.
#[derive(Debug, Clone)] // Clone provides cheap Arc clones
pub struct TagEngine {
    tags: Arc<DashMap<String, Tag>>,
}

impl TagEngine {
    pub fn new() -> Self {
        TagEngine {
            tags: Arc::new(DashMap::new()),
        }
    }

    /// Add or update a tag definition.
    /// (In a real scenario, this might load from config initially).
    pub fn register_tag(&self, tag: Tag) {
        self.tags.insert(tag.path.clone(), tag);
    }

    /// Get a snapshot of a tag's value.
    pub fn read_tag(&self, tag_path: &str) -> Option<TagValue> {
        self.tags.get(tag_path).map(|tag_ref| tag_ref.value().clone())
    }

    /// Update the value of an existing tag.
    pub fn update_tag_value(&self, tag_path: &str, new_value: TagValue) -> bool {
        match self.tags.get_mut(tag_path) {
            Some(mut tag_ref) => {
                tag_ref.value = new_value;
                true // Update successful
            }
            None => false, // Tag not found
        }
    }

    /// Get a list of all registered tag paths.
    pub fn get_all_tag_paths(&self) -> Vec<String> {
        self.tags.iter().map(|entry| entry.key().clone()).collect()
    }

    // TODO: Add methods for bulk reads/writes if needed
    // TODO: Add methods for browsing/querying tags
    // TODO: Integrate with persistence/historian
}

impl Default for TagEngine {
    fn default() -> Self {
        Self::new()
    }
}
