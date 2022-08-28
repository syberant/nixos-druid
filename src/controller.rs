use std::marker::PhantomData;

use crate::delegate::FOCUS_OPTION;

use crate::tree_node::TreeOptionNode;
use druid::widget::{Controller, Widget};
use druid::{Env, Event, EventCtx, Lens};

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

pub struct DisabledController<A, L> {
    lens: L,
    phantom_a: PhantomData<A>,
}

impl<A,L> DisabledController<A,L> {
    pub fn new(lens: L) -> Self {
        Self {
            lens,
            phantom_a: PhantomData,
        }
    }
}

impl<A, L: Lens<A, bool>, W: Widget<A>> Controller<A, W> for DisabledController<A, L> {
    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &A,
        env: &Env,
    ) {
        match event {
            druid::LifeCycle::WidgetAdded => {
                let show = self.lens.with(data, |show: &bool| *show);
                ctx.set_disabled(show);
                child.lifecycle(ctx, event, data, env);
            }
            _ => child.lifecycle(ctx, event, data, env),
        }
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut druid::UpdateCtx,
        old_data: &A,
        data: &A,
        env: &Env,
    ) {
        let new = self.lens.with(data, |show: &bool| *show);
        let old = self.lens.with(old_data, |show: &bool| *show);

        if new != old {
            ctx.set_disabled(new);
        }

        child.update(ctx, old_data, data, env);
    }
}
