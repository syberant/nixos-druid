mod node;
use node::OptionNode;

// Copyright 2022 The Druid Authors, Sybrand Aarnoutse.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// This is a pseudo tree file manager (no interaction with your actual
// filesystem whatsoever). It's intended to use most of the features of
// the `Tree` widget in a familiar context. It's by no mean polished, and
// probably lacks a lot of features, we want to focus on the tree widget here.

use nixos_druid::data::{AppData, DisplayData};
use nixos_druid::delegate::Delegate;
use nixos_druid::delegate::FOCUS_OPTION;

use druid::kurbo::Size;
use druid::widget::{Flex, Label, Scroll, Split};
use druid::{
    AppLauncher, BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    LocalizedString, PaintCtx, Point, UpdateCtx, Widget, WidgetExt, WidgetPod, WindowDesc,
};
use druid_widget_nursery::tree::{Tree, TreeNode, TREE_CHILD_SHOW, TREE_CHROOT};

use druid_widget_nursery::selectors;

selectors! {
    /// Command sent by the context menu to chroot to the targeted directory
    CHROOT,

    /// Internal wiring, mostly to update the filetype and the sorting
    UPDATE_DIR_VIEW,
    UPDATE_FILE,
}

// AttrsOf(Submodule(_)), ListOf(Submodule(_)) get an extra child for showing documentation
impl TreeNode for OptionNode {
    fn get_child(&self, index: usize) -> &Self {
        match (self.extra_child.as_ref(), index) {
            (Some(ref c), 0) => &c,
            (Some(_), i) => &self.children[i - 1],
            (None, i) => &self.children[i],
        }
    }

    fn for_child_mut(&mut self, index: usize, mut cb: impl FnMut(&mut Self, usize)) {
        match (self.extra_child.as_mut(), index) {
            (Some(ref mut c), 0) => cb(&mut *c, 0),
            (Some(_), i) => cb(&mut self.children[i - 1], i),
            (None, i) => cb(&mut self.children[i], i),
        }
    }

    fn children_count(&self) -> usize {
        match self.extra_child {
            Some(_) => self.children.len() + 1,
            None => self.children.len(),
        }
    }
}

struct OptionNodeWidget(WidgetPod<OptionNode, Flex<OptionNode>>);

impl OptionNodeWidget {
    fn new() -> Self {
        Self(WidgetPod::new(
            Flex::row()
                .with_default_spacer()
                .with_child(Label::dynamic(|data: &OptionNode, _env| data.name.clone())),
        ))
    }
}

impl Widget<OptionNode> for OptionNodeWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut OptionNode, env: &Env) {
        let new_event = match event {
            Event::MouseDown(ref mouse) if mouse.button.is_left() => {
                ctx.submit_command(FOCUS_OPTION.with(DisplayData::new_with(
                    data.documentation.clone(),
                    data.value.clone(),
                )));

                // Event handled, don't propagate
                None
            }
            Event::Command(cmd) if cmd.is(TREE_CHILD_SHOW) => None,
            Event::Command(cmd) if cmd.is(CHROOT) => {
                ctx.submit_notification(TREE_CHROOT);
                None
            }
            _ => Some(event),
        };

        if let Some(evt) = new_event {
            self.0.event(ctx, evt, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &OptionNode,
        env: &Env,
    ) {
        self.0.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: &OptionNode,
        data: &OptionNode,
        env: &Env,
    ) {
        self.0.update(ctx, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &OptionNode,
        env: &Env,
    ) -> Size {
        let size = self.0.layout(ctx, bc, data, env);
        self.0.set_origin(ctx, data, env, Point::ORIGIN);
        ctx.set_paint_insets(self.0.paint_insets());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &OptionNode, env: &Env) {
        self.0.paint(ctx, data, env)
    }
}

fn ui_builder() -> impl Widget<AppData<OptionNode>> {
    let tree = Tree::new(
        || OptionNodeWidget::new(),
        // The boolean deciding whether the tree should expand or not, acquired via Lens
        OptionNode::expanded,
    )
    .lens(AppData::tree);

    let wrapped_tree = Scroll::new(tree);
    let label = Label::dynamic(|data: &DisplayData, _| data.to_string()).lens(AppData::display);

    Split::columns(wrapped_tree, label)
        .split_point(0.3)
        .min_size(300.0, 400.0)
}

pub fn main() {
    // Create the main window
    let main_window = WindowDesc::new(ui_builder())
        .window_size((600.0, 600.0))
        .title(
            LocalizedString::new("nixos-config-browser-window-title")
                .with_placeholder("NixOS Config Browser"),
        );

    let option_root = nixos_druid::run::get_options().expect("Getting NixOS options failed");
    eprintln!("Parsing options is done.");
    let config_root = nixos_druid::run::get_config().expect("Getting NixOS config failed");
    eprintln!("Parsing config is done.");
    let root_name = "NixOS Configuration".to_string();
    let mut option_tree = OptionNode::new(root_name, option_root);
    option_tree.add_config(Some(config_root));

    let data = AppData::new(option_tree);
    eprintln!("GUI `Data` is built.");

    // start the application
    AppLauncher::with_window(main_window)
        .delegate(Delegate::new())
        // .log_to_console()
        .launch(data)
        .expect("launch failed");
}
