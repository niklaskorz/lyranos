// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

mod codetext;

use codetext::CodeText;

use druid::widget::prelude::*;
use druid::widget::TextBox;
use druid::FontDescriptor;
use druid::FontFamily;
use druid::{
    AppDelegate, AppLauncher, Color, Command, Data, DelegateCtx, Handled, Lens, LocalizedString,
    Menu, Selector, Target, Widget, WidgetExt, WindowDesc, WindowId,
};

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Code Editor");

const TEXT: &str = "import antigravity

a = 42.5
x = f\"Hello {a + 1}\"

def scope_test():
    def do_local():
        spam = 'local spam'

    def do_nonlocal():
        nonlocal spam
        spam = 'nonlocal spam'

    def do_global():
        global spam
        spam = 'global spam'

    spam = 'test spam'
    do_local()
    print('After local assignment:', spam)
    do_nonlocal()
    print('After nonlocal assignment:', spam)
    do_global()
    print('After global assignment:', spam)

scope_test()
print('In global scope:', spam)";

const OPEN_LINK: Selector<String> = Selector::new("druid-example.open-link");

#[derive(Clone, Data, Lens)]
struct AppState {
    code: CodeText,
}

struct Delegate;

impl<T: Data> AppDelegate<T> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        _data: &mut T,
        _env: &Env,
    ) -> Handled {
        if let Some(url) = cmd.get(OPEN_LINK) {
            #[cfg(not(target_arch = "wasm32"))]
            open::that_in_background(url);
            #[cfg(target_arch = "wasm32")]
            tracing::warn!("opening link({}) not supported on web yet.", url);
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title(WINDOW_TITLE)
        .menu(make_menu)
        .window_size((700.0, 600.0));

    // create the initial app state
    let initial_state = AppState {
        code: CodeText::new(TEXT.to_owned()),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .configure_env(|env, _app_state| {
            env.set(
                druid::theme::BACKGROUND_LIGHT,
                Color::from_hex_str("#282c34").unwrap(),
            );
        })
        .log_to_console()
        .delegate(Delegate)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    let textbox = TextBox::multiline()
        .with_font(FontDescriptor::new(FontFamily::MONOSPACE).with_size(16.0))
        .lens(AppState::code)
        .expand()
        .padding(5.0);
    textbox
}

#[allow(unused_assignments, unused_mut)]
fn make_menu<T: Data>(_window_id: Option<WindowId>, _app_state: &AppState, _env: &Env) -> Menu<T> {
    let mut base = Menu::empty();
    #[cfg(target_os = "macos")]
    {
        base = base.entry(druid::platform_menus::mac::application::default())
    }
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "openbsd"))]
    {
        base = base.entry(druid::platform_menus::win::file::default());
    }
    base.entry(
        Menu::new(LocalizedString::new("common-menu-edit-menu"))
            .entry(druid::platform_menus::common::undo())
            .entry(druid::platform_menus::common::redo())
            .separator()
            .entry(druid::platform_menus::common::cut().enabled(false))
            .entry(druid::platform_menus::common::copy())
            .entry(druid::platform_menus::common::paste()),
    )
}
