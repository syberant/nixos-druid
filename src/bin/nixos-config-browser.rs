mod node;
use node::{OptionDocumentation, OptionNode, OptionType};

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
use std::cmp::Ordering;
use std::sync::Arc;

use nix_druid::parse::{NixOption, NixSet, NixTypeValue, NixValue};

use druid::im::Vector;
use druid::kurbo::Size;
use druid::widget::{Flex, Label, Scroll, Split};
use druid::{
    AppLauncher, ArcStr, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, Lens, LifeCycle,
    LifeCycleCtx, LocalizedString, Menu, MenuItem, PaintCtx, Point, Selector, Target, UpdateCtx,
    Widget, WidgetExt, WidgetId, WidgetPod, WindowDesc,
};
use druid_widget_nursery::tree::{
    ChrootStatus, Tree, TreeNode, TREE_ACTIVATE_NODE, TREE_CHILD_SHOW, TREE_CHROOT, TREE_CHROOT_UP,
    TREE_NOTIFY_CHROOT, TREE_NOTIFY_PARENT,
};

use druid_widget_nursery::selectors;

/// Open this option in the option editor
const FOCUS_OPTION: Selector<OptionDocumentation> = Selector::new("main.focus-option");

selectors! {
    /// Command sent by the context menu to chroot to the targeted directory
    CHROOT,

    /// Internal wiring, mostly to update the filetype and the sorting
    UPDATE_DIR_VIEW,
    UPDATE_FILE,
}

impl TreeNode for OptionNode {
    fn get_child(&self, index: usize) -> &Self {
        // Don't handle submodules for now
        &self.children[index]
    }
    fn for_child_mut(&mut self, index: usize, mut cb: impl FnMut(&mut Self, usize)) {
        // Don't handle submodules for now
        cb(&mut self.children[index], index);
    }

    fn children_count(&self) -> usize {
        // Don't handle submodules for now
        self.children.len()
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
                if let Some(ref doc) = data.documentation {
                    ctx.submit_notification(FOCUS_OPTION.with(doc.clone()));

                    // Event handled, don't propagate
                    None
                } else {
                    Some(event)
                }
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

#[derive(Clone, Data, Lens)]
struct UiData {
    tree: OptionNode,
    text: String,
}

impl UiData {
    fn new(tree: OptionNode) -> Self {
        Self {
            tree,
            text: String::new(),
        }
    }
}

struct NotificationHandlingWidget<T> {
    inner: T,
}

impl<T> NotificationHandlingWidget<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Widget<UiData>> Widget<UiData> for NotificationHandlingWidget<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut UiData, env: &Env) {
        let event = match event {
            Event::Notification(notif) if notif.is(FOCUS_OPTION) => {
                if let Some(doc) = notif.get(FOCUS_OPTION) {
                    data.text = doc.to_string();
                }

                // Stop propagating to ancestors
                ctx.set_handled();
                None
            }
            x => Some(x),
        };

        if let Some(ev) = event {
            self.inner.event(ctx, ev, data, env);
        }
    }

    // Just pass all these function calls directly to the inner widget
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &UiData, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
    }
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &UiData, data: &UiData, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &UiData,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, data, env)
    }
    fn paint(&mut self, ctx: &mut PaintCtx, data: &UiData, env: &Env) {
        self.inner.paint(ctx, data, env);
    }
}

fn ui_builder() -> impl Widget<UiData> {
    let tree = Tree::new(
        || OptionNodeWidget::new(),
        // The boolean deciding whether the tree should expand or not, acquired via Lens
        OptionNode::expanded,
    )
    .lens(UiData::tree);

    let wrapped_tree = Scroll::new(tree);
    let label = Label::dynamic(|data: &String, _| data.to_string()).lens(UiData::text);

    NotificationHandlingWidget::new(
        Split::columns(wrapped_tree, label)
            .split_point(0.3)
            .min_size(300.0, 400.0),
    )
}

pub fn main() {
    // Create the main window
    let main_window = WindowDesc::new(ui_builder())
        .window_size((600.0, 600.0))
        .title(LocalizedString::new("tree-demo-window-title").with_placeholder("Tree Demo"));

    let option_root = nix_druid::run::get_options();
    let config_root = nix_druid::run::get_config();
    eprintln!("Parsing is done.");
    let root_name = "NixOS Configuration".to_string();
    let option_tree = OptionNode::new(root_name, option_root);

    let data = UiData::new(option_tree);
    eprintln!("GUI `Data` is built.");

    // start the application
    AppLauncher::with_window(main_window)
        // .log_to_console()
        .launch(data)
        .expect("launch failed");
}
