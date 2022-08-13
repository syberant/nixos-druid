mod node;
use node::{OptionDocumentation, OptionNode};

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

use nixos_druid::parse::NixGuardedValue;

use druid::kurbo::Size;
use druid::widget::{Flex, Label, Scroll, Split};
use druid::{
    AppDelegate, AppLauncher, BoxConstraints, Command, Data, DelegateCtx, Env, Event, EventCtx,
    Handled, LayoutCtx, Lens, LifeCycle, LifeCycleCtx, LocalizedString, PaintCtx, Point, Selector,
    Target, UpdateCtx, Widget, WidgetExt, WidgetPod, WindowDesc,
};
use druid_widget_nursery::tree::{
    Tree, TreeNode, TREE_ACTIVATE_NODE, TREE_CHILD_SHOW, TREE_CHROOT,
};

use druid_widget_nursery::selectors;

/// Open this option in the option editor
const FOCUS_OPTION: Selector<DisplayData> = Selector::new("main.focus-option");

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
        if let Some(ref nested) = self.extra_child {
            nested.get_child(index)
        } else {
            &self.children[index]
        }
    }

    fn for_child_mut(&mut self, index: usize, mut cb: impl FnMut(&mut Self, usize)) {
        if let Some(ref mut nested) = self.extra_child {
            nested.for_child_mut(index, cb)
        } else {
            cb(&mut self.children[index], index);
        }
    }

    fn children_count(&self) -> usize {
        if let Some(ref nested) = self.extra_child {
            nested.children_count()
        } else {
            self.children.len()
        }
    }
}

struct OptionNodeWidget(WidgetPod<OptionNode, Flex<OptionNode>>);

impl OptionNodeWidget {
    fn new() -> Self {
        Self(WidgetPod::new(
            Flex::row().with_default_spacer().with_child(Label::dynamic(
                |data: &OptionNode, _env| {
                    if let Some(ref t) = data.option_type {
                        if let Some(ext) = t.get_name_extension() {
                            return format!("{}.{}", data.name, ext);
                        }
                    }

                    data.name.clone()
                },
            )),
        ))
    }
}

impl Widget<OptionNode> for OptionNodeWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut OptionNode, env: &Env) {
        let new_event = match event {
            Event::MouseDown(ref mouse) if mouse.button.is_left() => {
                ctx.submit_command(FOCUS_OPTION.with(DisplayData {
                    documentation: data.documentation.clone(),
                    value: data.value.clone(),
                }));

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

/// Opener is the opener widget, the small icon the user interacts with to
/// expand directories.
struct Opener {
    label: WidgetPod<String, Label<String>>,
}

impl Opener {
    // TODO: Nice icons
    fn label(&self, data: &OptionNode) -> String {
        if let Some(ref t) = data.option_type {
            if t.has_nested_submodule() {
                if data.expanded {
                    "📖"
                } else {
                    "📕"
                }
            } else {
                match data.name.as_ref() {
                    "enable" => "✅",
                    _ => "⚙️",
                }
            }
        } else {
            if data.expanded {
                "📂"
            } else {
                "📁"
            }
        }
        .to_owned()
    }
}

impl Widget<OptionNode> for Opener {
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, data: &mut OptionNode, _env: &Env) {
        if data.is_branch() {
            match event {
                // The wrapping tree::Opener widget transforms a click to this command.
                Event::Command(cmd) if cmd.is(TREE_ACTIVATE_NODE) => {
                    // We care only for branches (we could of course imagine interactions with files too)
                    if data.is_branch() {
                        data.expanded = !data.expanded;
                    }
                }
                _ => (),
            }
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &OptionNode,
        env: &Env,
    ) {
        let label = self.label(data);
        self.label.lifecycle(ctx, event, &label, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &OptionNode, data: &OptionNode, env: &Env) {
        if old_data.expanded != data.expanded {
            let label = self.label(data);
            self.label.update(ctx, &label, env);
        }
        if data.is_branch() {
            self.label.update(ctx, &self.label(data), env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &OptionNode,
        env: &Env,
    ) -> Size {
        let label = self.label(data);
        let size = self.label.layout(ctx, bc, &label, env);
        self.label.set_origin(ctx, &label, env, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &OptionNode, env: &Env) {
        let label = self.label(data);
        self.label.paint(ctx, &label, env)
    }
}

#[derive(Clone, Data, Lens)]
struct DisplayData {
    documentation: Option<OptionDocumentation>,
    // FIXME: Very hacky, maybe implement PartialEq?
    #[data(ignore)]
    value: Option<NixGuardedValue>,
}

impl std::fmt::Display for DisplayData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.documentation.as_ref(), self.value.as_ref()) {
            (None, _) => write!(f, "No documentation available."),
            (Some(ref d), Some(ref v)) => write!(f, "Value: {}\n\n\n{}", v, d),
            (Some(ref d), None) => d.fmt(f),
        }
    }
}

impl DisplayData {
    fn new() -> Self {
        Self {
            documentation: None,
            value: None,
        }
    }
}

#[derive(Clone, Data, Lens)]
struct UiData {
    tree: OptionNode,
    display: DisplayData,
}

impl UiData {
    fn new(tree: OptionNode) -> Self {
        Self {
            tree,
            display: DisplayData::new(),
        }
    }
}

struct Delegate;

impl AppDelegate<UiData> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut UiData,
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

fn ui_builder() -> impl Widget<UiData> {
    let tree = Tree::new(
        || OptionNodeWidget::new(),
        // The boolean deciding whether the tree should expand or not, acquired via Lens
        OptionNode::expanded,
    )
    .with_opener(|| Opener {
        label: WidgetPod::new(Label::dynamic(|st: &String, _| st.clone())),
    })
    .lens(UiData::tree);

    let wrapped_tree = Scroll::new(tree);
    let label = Label::dynamic(|data: &DisplayData, _| data.to_string()).lens(UiData::display);

    Split::columns(wrapped_tree, label)
        .split_point(0.3)
        .min_size(300.0, 400.0)
}

pub fn main() {
    // Create the main window
    let main_window = WindowDesc::new(ui_builder())
        .window_size((600.0, 600.0))
        .title(
            LocalizedString::new("nixos-option-browser-window-title")
                .with_placeholder("NixOS Options Browser"),
        );

    let root = nixos_druid::run::get_options();
    eprintln!("Parsing options is done.");
    let root_name = "NixOS Configuration".to_string();
    let tree = OptionNode::new(root_name, root);

    let data = UiData::new(tree);
    eprintln!("GUI `Data` is built.");

    // start the application
    AppLauncher::with_window(main_window)
        .delegate(Delegate)
        // .log_to_console()
        .launch(data)
        .expect("launch failed");
}
