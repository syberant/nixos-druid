use crate::data::{AppData, DisplayData};
use crate::tree_node::TreeOptionNode;
use druid::{AppDelegate, Command, DelegateCtx, Env, Handled, Selector, Target};
use std::marker::PhantomData;

/// Open this option in the option editor
pub const FOCUS_OPTION: Selector<DisplayData> = Selector::new("main.focus-option");

pub struct Delegate<T>(PhantomData<T>);

impl<T> Delegate<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: TreeOptionNode> AppDelegate<AppData<T>> for Delegate<T> {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppData<T>,
        _env: &Env,
    ) -> Handled {
        if let Some(doc) = cmd.get(FOCUS_OPTION) {
            data.display = doc.clone();
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
