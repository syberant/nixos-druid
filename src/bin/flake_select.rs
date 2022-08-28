// This entire file is quite horrible honestly but it (mostly) works,
// is nicely isolated and can easily be replaced in the future.

fn main() {}

use druid::commands::SHOW_OPEN_PANEL;
use druid::widget::{Button, Controller, Flex, Label, TextBox, ViewSwitcher, Widget};
use druid::{
    AppDelegate, AppLauncher, Data, FileDialogOptions, Lens, LensExt, LocalizedString, Selector,
    WidgetExt, WindowDesc,
};
use druid_widget_nursery::ListSelect;
use nixos_druid::controller::DisabledController;
use nixos_druid::run::get_available_nixos_configurations;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Data, Lens)]
pub struct SelectData {
    flake_location: String,
    hostname: Option<String>,
    /// This is just used to export the data so we can use it after the window is closed.
    #[data(ignore)]
    output: Option<Rc<RefCell<SelectData>>>,
}

// Implement Debug ourselves to avoid recursive printing via `SelectData::output`
impl std::fmt::Debug for SelectData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectData")
            .field("flake_location", &self.flake_location)
            .field("hostname", &self.hostname)
            .finish_non_exhaustive()
    }
}

impl SelectData {
    pub fn extract_results(self) -> Option<(String, String)> {
        match self.hostname.as_ref() {
            Some(n) => Some((self.flake_location.to_string(), n.to_string())),
            None => None,
        }
    }
}

struct SelectionDelegate;

impl AppDelegate<SelectData> for SelectionDelegate {
    fn command(
        &mut self,
        _ctx: &mut druid::DelegateCtx,
        _target: druid::Target,
        cmd: &druid::Command,
        data: &mut SelectData,
        _env: &druid::Env,
    ) -> druid::Handled {
        use druid::commands::OPEN_FILE;
        use druid::Handled::*;

        match cmd {
            cmd if cmd.is(OPEN_FILE) => {
                let info = cmd.get(OPEN_FILE).unwrap();
                let path = info.path.to_str().unwrap().to_string();

                data.flake_location = path;

                Yes
            }
            _ => No,
        }
    }
}

// TODO: Use controller with `LifeCycle::WidgetAdded` to eliminate argument `data` here
fn ui_builder() -> impl Widget<SelectData> {
    let select_flake = Flex::row()
        .with_child(Label::new("Select the location of your system flake: "))
        .with_child(
            // TextBox::new().with_placeholder("/etc/nixos")
            // Or use a file dialog, but I think a TextBox may be both easier and nicer here
            Button::dynamic(|data: &String, _| {
                if get_available_nixos_configurations(data.as_ref()).is_none() {
                    "Choose flake".to_string()
                } else {
                    data.to_string()
                }
            })
            .on_click(|ctx, _data, _env| {
                // Select a file
                let options = FileDialogOptions::new()
                    .select_directories()
                    .title("Select location of system flake");
                ctx.submit_command(SHOW_OPEN_PANEL.with(options))
            })
            .lens(SelectData::flake_location),
        );

    let select_hostname = Flex::row()
        .with_child(Label::new("Select the particular `nixosConfiguration`: "))
        .with_child(ViewSwitcher::new(
            |dat: &SelectData, _env| dat.flake_location.clone(),
            |path: &String, _dat, _env| {
                let mut list: Vec<(String, Option<String>)> = vec![("---".to_string(), None)];

                if let Some(available) = get_available_nixos_configurations(path.as_ref()) {
                    list.extend(
                        available
                            .into_iter()
                            .map(|host: String| (host.clone(), Some(String::from(host)))),
                    )
                }

                Box::new(ListSelect::new(list).lens(SelectData::hostname))
            },
        ));

    let control_buttons = Flex::row()
        .with_child(
            Button::new("Select").on_click(|ctx, data: &mut SelectData, _env| {
                // This assumes we are outputting valid data
                // If data is invalid this button should be "disabled" which is the controllers job
                let new = data.clone();
                *data
                    .output
                    .as_ref()
                    .expect("`Data` given to launch flake selection window does not have an output")
                    .as_ref()
                    .borrow_mut() = new;

                ctx.window().close();
            }),
        )
        .controller(DisabledController::new(
            SelectData::hostname.map(|x| x.is_none(), |_, _| unimplemented!()),
        ));

    Flex::column()
        .with_child(select_flake)
        .with_child(select_hostname)
        .with_child(control_buttons)
}

pub fn select_hostname() -> SelectData {
    let output = Rc::new(RefCell::new(SelectData {
        hostname: None,
        flake_location: String::new(),
        output: None,
    }));
    let data = SelectData {
        hostname: None,
        flake_location: String::new(),
        output: Some(Rc::clone(&output)),
    };

    let window = WindowDesc::new(ui_builder())
        .window_size((400.0, 400.0))
        .title(
            LocalizedString::new("nixos-select-window-title")
                .with_placeholder("nixos-druid: Select hostname"),
        );

    AppLauncher::with_window(window)
        .delegate(SelectionDelegate)
        .launch(data)
        .expect("Launching selection window failed");

    let res = output.as_ref().borrow().clone();
    res
}
