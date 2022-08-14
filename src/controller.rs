use std::marker::PhantomData;

use crate::delegate::FOCUS_OPTION;

use crate::tree_node::TreeOptionNode;
use druid::widget::{Controller, Widget};
use druid::{Env, Event, EventCtx};

pub struct FocusOption<T>(PhantomData<T>);

impl<T> FocusOption<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: TreeOptionNode, W: Widget<T>> Controller<T, W> for FocusOption<T> {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let new_event = match event {
            Event::MouseDown(ref mouse) if mouse.button.is_left() => {
                ctx.submit_command(FOCUS_OPTION.with(data.focused_display_data()));

                // Event handled, don't propagate
                None
            }
            _ => Some(event),
        };

        if let Some(evt) = new_event {
            child.event(ctx, evt, data, env);
        }
    }
}
