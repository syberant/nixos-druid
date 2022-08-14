use crate::tree_node::TreeOptionNode;
use druid::widget::Label;
use druid::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point,
    Size, UpdateCtx, Widget, WidgetPod,
};
use druid_widget_nursery::tree::TREE_ACTIVATE_NODE;

/// Opener is the opener widget, the small icon the user interacts with to
/// expand directories.
pub struct Opener {
    label: WidgetPod<String, Label<String>>,
}

impl Opener {
    pub fn new() -> Self {
        Self {
            label: WidgetPod::new(Label::dynamic(|st: &String, _| st.clone())),
        }
    }
}

impl<T: TreeOptionNode> Widget<T> for Opener {
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, data: &mut T, _env: &Env) {
        if data.is_branch() {
            match event {
                // The wrapping tree::Opener widget transforms a click to this command.
                Event::Command(cmd) if cmd.is(TREE_ACTIVATE_NODE) => {
                    // We care only for branches (we could of course imagine interactions with files too)
                    if data.is_branch() {
                        data.toggle_expanded();
                    }
                }
                _ => (),
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let label = data.get_icon();
        self.label.lifecycle(ctx, event, &label, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if old_data.is_expanded() != data.is_expanded() {
            let label = data.get_icon();
            self.label.update(ctx, &label, env);
        }
        if data.is_branch() {
            self.label.update(ctx, &data.get_icon(), env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let label = data.get_icon();
        let size = self.label.layout(ctx, bc, &label, env);
        self.label.set_origin(ctx, &label, env, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let label = data.get_icon();
        self.label.paint(ctx, &label, env)
    }
}
