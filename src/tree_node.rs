use druid_widget_nursery::TreeNode;

pub trait TreeOptionNode: TreeNode {
    fn get_icon(&self) -> String;

    fn is_expanded(&self) -> bool;
    fn toggle_expanded(&mut self);
}
