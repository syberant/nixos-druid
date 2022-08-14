use druid_widget_nursery::TreeNode;
use crate::data::DisplayData;

pub trait TreeOptionNode: TreeNode {
    fn get_icon(&self) -> String;
    fn focused_display_data(&self) -> DisplayData;

    fn is_expanded(&self) -> bool;
    fn toggle_expanded(&mut self);
}
