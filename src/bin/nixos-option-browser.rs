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
use nixos_druid::view::Opener;

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
    .with_opener(|| Opener::new())
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
            LocalizedString::new("nixos-option-browser-window-title")
                .with_placeholder("NixOS Options Browser"),
        );

    let root = nixos_druid::run::get_options().expect("Getting NixOS options failed");
    eprintln!("Parsing options is done.");
    let root_name = "NixOS Configuration".to_string();
    let tree = OptionNode::new(root_name, root);

    let data = AppData::new(tree);
    eprintln!("GUI `Data` is built.");

    // start the application
    AppLauncher::with_window(main_window)
        .delegate(Delegate::new())
        // .log_to_console()
        .launch(data)
        .expect("launch failed");
}
