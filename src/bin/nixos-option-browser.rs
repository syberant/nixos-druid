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
const FOCUS_OPTION: Selector<NodeType> = Selector::new("main.focus-option");

selectors! {
    /// Command sent by the context menu to chroot to the targeted directory
    CHROOT,

    /// Internal wiring, mostly to update the filetype and the sorting
    UPDATE_DIR_VIEW,
    UPDATE_FILE,
}

#[derive(Clone, Data, Debug)]
enum NodeType {
    DocumentedSet(LeafOption),
    Set,
    Option(LeafOption),
}

#[derive(Clone, Data, Debug)]
struct LeafOption {
    description: String,
    type_name: String,
    default: Option<String>,
    example: Option<String>,
}

impl From<NixOption> for LeafOption {
    fn from(opt: NixOption) -> Self {
        Self {
            description: opt.description,
            type_name: opt.r#type.to_string(),
            default: opt.default.map(|s| s.to_string()),
            example: opt.example.map(|s| s.to_string()),
        }
    }
}

impl std::fmt::Display for LeafOption {
    // TODO: Clean this mess
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Description: {}\n\nType: {}",
            self.description, self.type_name
        )?;

        if let Some(ref def) = self.default {
            write!(f, "\n\nDefault: {}", def)?;
        }
        if let Some(ref ex) = self.example {
            write!(f, "\n\nExample: {}", ex)?;
        }

        write!(f, "")
    }
}

#[derive(Clone, Lens, Debug, Data)]
struct FSNode {
    /// Name of the node, that is diplayed in the tree
    name: ArcStr,
    #[data(same_fn = "children_same")]
    children: Vector<FSNode>,
    /// Explicit storage of the type (file or directory)
    node_type: NodeType,
    /// Keep track of the expanded state
    expanded: bool,
    /// Keep track of the chroot state (see TreeNode::get_chroot for description of the chroot mechanism)
    chroot_: Option<usize>,
}
/// Children FSNodes. We wrap them in an Arc to avoid a ugly side effect of Vector (discussed in examples/tree.rs)
fn children_same(lhs: &Vector<FSNode>, rhs: &Vector<FSNode>) -> bool {
    lhs.len() == rhs.len() && lhs.iter().zip(rhs.iter()).all(|(a, b)| a.same(b))
}

/// We use FSNode as a tree node, implementing the TreeNode trait.
impl FSNode {
    fn new(name: String, option: LeafOption) -> Self {
        FSNode {
            name: ArcStr::from(name),
            children: Vector::new(),
            node_type: NodeType::Option(option),
            expanded: false,
            chroot_: None,
        }
    }

    fn new_dir(name: String, children: &NixSet) -> Self {
        let children = children
            .into_iter()
            .map(|(k, v): (&String, &Box<NixValue>)| create_node(v, k.to_string()))
            .collect();

        let mut node = FSNode {
            name: ArcStr::from(name),
            children,
            node_type: NodeType::Set,
            expanded: false,
            chroot_: None,
        };
        node.sort();
        return node;
    }

    fn new_documented(name: String, option: LeafOption, children: &NixSet) -> Self {
        let children = children
            .into_iter()
            .map(|(k, v): (&String, &Box<NixValue>)| create_node(v, k.to_string()))
            .collect();

        let mut node = FSNode {
            name: ArcStr::from(name),
            children,
            node_type: NodeType::DocumentedSet(option),
            expanded: false,
            chroot_: None,
        };
        node.sort();
        return node;
    }

    /// The sorting is directories first and alphanumeric order.
    /// This is called upon insertion or update of a child, by the
    /// FSNodeWidget.
    fn sort(&mut self) {
        use Ordering::*;

        self.children
            .sort_by(|a, b| match (&a.node_type, &b.node_type) {
                (NodeType::Option(_), NodeType::Set) => Greater,
                (NodeType::Option(_), NodeType::DocumentedSet(_)) => Greater,
                (NodeType::Set, NodeType::Option(_)) => Less,
                (NodeType::DocumentedSet(_), NodeType::Option(_)) => Less,

                _ => match (a.name.as_ref(), b.name.as_ref()) {
                    (_, "") => Less,
                    ("", _) => Greater,
                    ("enable", _) => Less,
                    (_, "enable") => Greater,
                    _ => a.name.cmp(&b.name),
                },
            });
    }
}

impl TreeNode for FSNode {
    fn children_count(&self) -> usize {
        self.children.len()
    }

    fn get_child(&self, index: usize) -> &FSNode {
        &self.children[index]
    }

    fn for_child_mut(&mut self, index: usize, mut cb: impl FnMut(&mut Self, usize)) {
        // Apply the closure to a clone of the child and update the `self.children` vector
        // with the clone iff it's changed to avoid unnecessary calls to `update(...)`
        // These calls currently come at a BIG performance penalty when opening a node with many children.

        // TODO: there must be a more idiomatic way to do this
        let orig = &self.children[index];
        let mut new = orig.as_ref().clone();
        cb(&mut new, index);
        if !orig.as_ref().same(&new) {
            self.children.remove(index);
            self.children.insert(index, Arc::new(new));
        }
    }

    fn is_branch(&self) -> bool {
        use NodeType::*;

        match &self.node_type {
            Set => true,
            DocumentedSet(_) => true,
            Option(_) => false,
        }
    }

    fn rm_child(&mut self, index: usize) {
        self.children.remove(index);
    }

    // those two accessors are the most simple implementation to enable chroot, and should
    // be enough for most use cases.
    fn chroot(&mut self, idx: Option<usize>) {
        self.chroot_ = idx;
    }

    fn get_chroot(&self) -> Option<usize> {
        self.chroot_
    }
}

/// FSOpener is the opener widget, the small icon the user interacts with to
/// expand directories.
struct FSOpener {
    label: WidgetPod<String, Label<String>>,
    chroot_status: ChrootStatus,
}

impl FSOpener {
    // TODO: Nice icons
    fn label(&self, data: &FSNode) -> String {
        use NodeType::*;

        if self.chroot_status == ChrootStatus::YES {
            // for the chroot we show that the user can move the virtual root up a dir
            "↖️"
        } else {
            match data.node_type {
                Option(_) => match data.name.as_ref() {
                    "enable" => "✅",
                    _ => "⚙️",
                },
                DocumentedSet(_) => {
                    if data.expanded {
                        "📖"
                    } else {
                        "📕"
                    }
                }
                Set => {
                    if data.expanded {
                        "📂"
                    } else {
                        "📁"
                    }
                }
            }
        }
        .to_owned()
    }
}

impl Widget<FSNode> for FSOpener {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut FSNode, _env: &Env) {
        if data.is_branch() {
            match event {
                // The wrapping tree::Opener widget transforms a click to this command.
                Event::Command(cmd) if cmd.is(TREE_ACTIVATE_NODE) => {
                    // We care only for branches (we could of course imagine interactions with files too)
                    if data.is_branch() {
                        match self.chroot_status {
                            // not on chroot ? expand
                            ChrootStatus::NO | ChrootStatus::ROOT => data.expanded = !data.expanded,
                            // on chroot ? chroot up
                            ChrootStatus::YES => ctx.submit_notification(TREE_CHROOT_UP),
                        }
                    }
                }
                // The Tree widget sends this command when the node's virtual root status change.
                // This is because the data of a virtual root is not enough to tell. We keep the
                // info on the widget at the moment.
                Event::Command(cmd) if cmd.is(TREE_NOTIFY_CHROOT) => {
                    let new_status = cmd.get(TREE_NOTIFY_CHROOT).unwrap().clone();
                    if self.chroot_status != new_status {
                        self.chroot_status = new_status;
                        if let ChrootStatus::YES = self.chroot_status {
                            data.expanded = true;
                        }
                        ctx.children_changed();
                        ctx.request_update();
                    }
                }
                _ => (),
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &FSNode, env: &Env) {
        let label = self.label(data);
        self.label.lifecycle(ctx, event, &label, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &FSNode, data: &FSNode, env: &Env) {
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
        data: &FSNode,
        env: &Env,
    ) -> Size {
        let label = self.label(data);
        let size = self.label.layout(ctx, bc, &label, env);
        self.label.set_origin(ctx, &label, env, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &FSNode, env: &Env) {
        let label = self.label(data);
        self.label.paint(ctx, &label, env)
    }
}

fn make_dir_context_menu(widget_id: WidgetId) -> Menu<FSNode> {
    Menu::empty().entry(MenuItem::new(LocalizedString::new("Chroot")).on_activate(
        move |ctx, _data: &mut FSNode, _env| {
            ctx.submit_command(CHROOT.to(Target::Widget(widget_id)));
        },
    ))
}

fn make_file_context_menu(_widget_id: WidgetId) -> Menu<FSNode> {
    Menu::empty()
}

/// THis is the user widget we pass to the Tree constructor, to display `FSNode`s
/// It is a variation of `druid::widget::Either` that displays a Label or a TextBox
/// according to `editing`.
pub struct FSNodeWidget {
    normal_branch: WidgetPod<FSNode, Flex<FSNode>>,
}

impl FSNodeWidget {
    #[allow(clippy::new_without_default)]
    pub fn new() -> FSNodeWidget {
        FSNodeWidget {
            normal_branch: WidgetPod::new(Flex::row().with_default_spacer().with_child(
                Label::dynamic(|data: &FSNode, _env| String::from(data.name.as_ref())),
            )),
        }
    }
}

impl Widget<FSNode> for FSNodeWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut FSNode, env: &Env) {
        // Well, a lot of stuff is going on here, but not directly related to the tree widget. It's
        // mostly plubming for the FSNodeWidget interactions... The exercise is left to the reader.
        // (i.e. I'm tired documenting, I'm not even sure how much this may change in a near future.)
        let new_event = match event {
            Event::MouseDown(ref mouse) if mouse.button.is_left() => {
                ctx.submit_notification(FOCUS_OPTION.with(data.node_type.clone()));

                // Event handled, don't propagate
                None
            }
            Event::MouseDown(ref mouse) if mouse.button.is_right() => {
                if data.is_branch() {
                    ctx.show_context_menu(make_dir_context_menu(ctx.widget_id()), mouse.pos);
                } else {
                    ctx.show_context_menu(make_file_context_menu(ctx.widget_id()), mouse.pos);
                }
                None
            }
            Event::Command(cmd) if cmd.is(UPDATE_FILE) => {
                ctx.submit_notification(TREE_NOTIFY_PARENT.with(UPDATE_DIR_VIEW));
                None
            }
            Event::Command(cmd) if cmd.is(TREE_CHILD_SHOW) => None,
            Event::Command(cmd) if cmd.is(CHROOT) => {
                ctx.submit_notification(TREE_CHROOT);
                None
            }
            Event::Command(cmd) if cmd.is(TREE_NOTIFY_PARENT) => {
                let cmd_data = cmd.get(TREE_NOTIFY_PARENT).unwrap();
                if *cmd_data == UPDATE_DIR_VIEW {
                    // data.update();
                    ctx.set_handled();
                    None
                } else {
                    Some(event)
                }
            }
            _ => Some(event),
        };
        if let Some(evt) = new_event {
            self.normal_branch.event(ctx, evt, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &FSNode, env: &Env) {
        self.normal_branch.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &FSNode, data: &FSNode, env: &Env) {
        self.normal_branch.update(ctx, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &FSNode,
        env: &Env,
    ) -> Size {
        let size = self.normal_branch.layout(ctx, bc, data, env);
        self.normal_branch.set_origin(ctx, data, env, Point::ORIGIN);
        ctx.set_paint_insets(self.normal_branch.paint_insets());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &FSNode, env: &Env) {
        self.normal_branch.paint(ctx, data, env)
    }
}

#[derive(Clone, Data, Lens)]
struct UiData {
    tree: FSNode,
    text: String,
}

impl UiData {
    fn new(tree: FSNode) -> Self {
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
                if let Some(NodeType::Option(opt)) = notif.get(FOCUS_OPTION) {
                    data.text = opt.to_string();
                }
                if let Some(NodeType::DocumentedSet(opt)) = notif.get(FOCUS_OPTION) {
                    data.text = opt.to_string();
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
        || {
            // Our items are editable. If editing is true, we show a TextBox of the name,
            // otherwise it's a Label
            FSNodeWidget::new()
        },
        // The boolean deciding whether the tree should expand or not, acquired via Lens
        FSNode::expanded,
    )
    .with_opener(|| FSOpener {
        label: WidgetPod::new(Label::dynamic(|st: &String, _| st.clone())),
        chroot_status: ChrootStatus::NO,
    })
    .lens(UiData::tree);

    let wrapped_tree = Scroll::new(tree);
    let label = Label::dynamic(|data: &String, _| data.to_string()).lens(UiData::text);

    NotificationHandlingWidget::new(
        Split::columns(wrapped_tree, label)
            .split_point(0.3)
            .min_size(300.0, 400.0),
    )
}

fn create_node(value: &NixValue, mut name: String) -> FSNode {
    match value {
        NixValue::Option(o) => {
            let head = LeafOption::from(o.clone());

            match o.r#type {
                NixTypeValue::Type(ref t) => match t.description.as_str() {
                    "attribute set of submodule" | "list of submodule" | "null or submodule" => {
                        match t.description.as_str() {
                            "attribute set of submodule" => name.push_str(".<name>"),
                            "list of submodule" => name.push_str(".*"),
                            _ => {}
                        }

                        FSNode::new_documented(
                            name,
                            head,
                            &t.nestedTypes["elemType"].get_submodule().unwrap().options,
                        )
                    }
                    _ => FSNode::new(name, head),
                },
                NixTypeValue::Submodule(ref s) => FSNode::new_documented(name, head, &s.options),
                _ => FSNode::new(name, head),
            }
        }
        NixValue::Set(s) => FSNode::new_dir(name, s),
    }
}

pub fn main() {
    // Create the main window
    let main_window = WindowDesc::new(ui_builder())
        .window_size((600.0, 600.0))
        .title(LocalizedString::new("tree-demo-window-title").with_placeholder("Tree Demo"));

    let root = nix_druid::run::get_options();
    let root_name = "NixOS Configuration".to_string();
    let option_tree = create_node(&root, root_name);

    let data = UiData::new(option_tree);

    // start the application
    AppLauncher::with_window(main_window)
        // .log_to_console()
        .launch(data)
        .expect("launch failed");
}
