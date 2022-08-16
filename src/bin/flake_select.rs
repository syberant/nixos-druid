// This entire file is quite horrible honestly but it (mostly) works,
// is nicely isolated and can easily be replaced in the future.

use druid::widget::{Button, Flex, Label, TextBox, Widget};
use druid::{AppLauncher, Data, Lens, LocalizedString, Selector, WidgetExt, WindowDesc};
use druid_widget_nursery::ListSelect;
use nixos_druid::run::get_available_nixos_configurations;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const RESET_HOSTNAMES: Selector = Selector::new("flake-select.reset-hostnames");

#[derive(Clone, Data, Lens)]
pub struct SelectData {
    flake_location: Arc<String>,
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

struct HostnameSelectorWidget(Box<dyn Widget<SelectData>>);

impl HostnameSelectorWidget {
    fn new_from_list(list: Vec<(String, Option<String>)>) -> Box<dyn Widget<SelectData>> {
        Box::new(ListSelect::new(list).lens(SelectData::hostname))
    }

    fn new_from_data(data: &SelectData) -> Self {
        let hostnames: Vec<(String, Option<String>)> = if let Some(available) =
            get_available_nixos_configurations(data.flake_location.as_ref())
        {
            available
                .into_iter()
                .map(|host: String| (host.clone(), Some(String::from(host))))
                .chain(vec![("Please select a hostname".to_string(), None)])
                .collect()
        } else {
            vec![("Please select a system flake".to_string(), None)]
        };

        Self(Self::new_from_list(hostnames))
    }
}

impl Widget<SelectData> for HostnameSelectorWidget {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut SelectData,
        env: &druid::Env,
    ) {
        let new_event = match event {
            druid::Event::Command(cmd) if cmd.is(RESET_HOSTNAMES) => {
                // Flake location changed, regenerate hostname picker
                *self = Self::new_from_data(data);
                // Old hostnames not valid anymore, reset to None
                data.hostname = None;

                ctx.set_handled();
                None
            }
            x => Some(x),
        };
        if let Some(ev) = new_event {
            self.0.event(ctx, ev, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &SelectData,
        env: &druid::Env,
    ) {
        self.0.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        old_data: &SelectData,
        data: &SelectData,
        env: &druid::Env,
    ) {
        if !data.flake_location.same(&old_data.flake_location) {
            ctx.submit_command(RESET_HOSTNAMES);
        }

        self.0.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &SelectData,
        env: &druid::Env,
    ) -> druid::Size {
        self.0.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &SelectData, env: &druid::Env) {
        self.0.paint(ctx, data, env);
    }
}

// TODO: Use controller with `LifeCycle::WidgetAdded` to eliminate argument `data` here
fn ui_builder(data: &SelectData) -> impl Widget<SelectData> {
    let select_flake = Flex::row()
        .with_child(Label::new("Select the location of your system flake: "))
        .with_child(
            TextBox::new()
            // Or use a file dialog, but I think a TextBox may be both easier and nicer here
            // Button::dynamic(|data: &ArcStr, _| data.to_string()).on_click(|ctx, _data, _env| {
                // // Select a file
                // let options = FileDialogOptions::new()
                    // .select_directories()
                    // .title("Select location of system flake");
                // ctx.submit_command(SHOW_OPEN_PANEL.with(options))
            // }),
        )
        .lens(SelectData::flake_location);

    let select_hostname = Flex::row()
        .with_child(Label::new("Select the particular `nixosConfiguration`: "))
        .with_child(HostnameSelectorWidget::new_from_data(data));

    let control_buttons = Flex::row().with_child(Button::new("Select").on_click(
        |ctx, data: &mut SelectData, _env| {
            if data.hostname != None {
                let new = data.clone();
                *data
                    .output
                    .as_ref()
                    .expect("`Data` given to launch flake selection window does not have an output")
                    .as_ref()
                    .borrow_mut() = new;

                ctx.window().close();
            }
        },
    ));

    Flex::column()
        .with_child(select_flake)
        .with_child(select_hostname)
        .with_child(control_buttons)
}

pub fn select_hostname() -> SelectData {
    let output = Rc::new(RefCell::new(SelectData {
        hostname: None,
        flake_location: Arc::new(String::new()),
        output: None,
    }));
    let data = SelectData {
        hostname: None,
        flake_location: Arc::new(String::from("/etc/nixos")),
        output: Some(Rc::clone(&output)),
    };

    let window = WindowDesc::new(ui_builder(&data))
        .window_size((400.0, 400.0))
        .title(
            LocalizedString::new("nixos-select-window-title")
                .with_placeholder("nixos-druid: Select hostname"),
        );

    AppLauncher::with_window(window)
        .launch(data)
        .expect("Launching selection window failed");

    let res = output.as_ref().borrow().clone();
    res
}
