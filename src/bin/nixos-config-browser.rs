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

use nixos_druid::controller::FocusOption;
use nixos_druid::data::{AppData, DisplayData};
use nixos_druid::delegate::Delegate;

use druid::widget::{Flex, Label, Scroll, Split};
use druid::{AppLauncher, LocalizedString, Widget, WidgetExt, WindowDesc};
use druid_widget_nursery::tree::{Tree, TreeNode};

use druid_widget_nursery::selectors;

selectors! {
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

fn ui_builder() -> impl Widget<AppData<OptionNode>> {
    let tree = Tree::new(
        || {
            Flex::row()
                .with_default_spacer()
                .with_child(Label::dynamic(|data: &OptionNode, _env| {
                    if let Some(ref t) = data.option_type {
                        if let Some(ext) = t.get_name_extension() {
                            return format!("{}.{}", data.name, ext);
                        }
                    }

                    data.name.clone()
                }))
                .controller(FocusOption::new())
        },
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
