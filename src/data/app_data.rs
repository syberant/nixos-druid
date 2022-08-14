use crate::data::DisplayData;
use crate::tree_node::TreeOptionNode;
use druid::{Data, Lens};

/// Top-level `Data` struct, holds ALL data of the application
#[derive(Clone, Data, Lens)]
pub struct AppData<T: TreeOptionNode> {
    tree: T,
    pub display: DisplayData,
}

impl<T: TreeOptionNode> AppData<T> {
    pub fn new(tree: T) -> Self {
        Self {
            tree,
            display: DisplayData::new(),
        }
    }
}
