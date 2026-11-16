use iced::Element;
use iced::widget::{container, responsive};
use otty_app_i18n as i18n;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::status_bar::{StatusBar, StatusItem};
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::size;

use super::model::{
    BlockCounter, CounterParts, CounterWidths, StatusBarViewModel,
};

/// Render the status bar at the bottom of the window.
///
/// The left side names the machine and, with open sessions, the folder path
/// of the focused pane and, while a block is selected there, the block
/// counter with the keys that move the selection. With open sessions the
/// right side counts panes and sessions. The program is left out: the
/// header above the focused pane names it. `OTTY <version>` always ends the
/// right side. In a narrow window the keys of the counter are left out
/// first, then the counter, so the bar stays one line.
pub(crate) fn view<'a, Message: Clone + 'a>(
    vm: &StatusBarViewModel,
    version: &str,
    theme: ThemeProps<'a>,
) -> Element<'a, Message> {
    let vm = vm.clone();
    let version = version.to_string();
    let palette = theme.theme.ui().clone();

    let bar =
        responsive(move |size| bar(&vm, &version, &palette, size.width).into());

    container(bar).height(size::STATUSBAR).into()
}

/// The status bar of `vm` in a bar `available` pixels wide. The bar is
/// built once without the block counter and measured; then the counter
/// goes at the end of the left side in full, without its keys, or not at
/// all, whatever keeps the bar on one line.
fn bar<Message: Clone>(
    vm: &StatusBarViewModel,
    version: &str,
    palette: &UiPalette,
    available: f32,
) -> StatusBar<Message> {
    let bar = bar_without_counter(vm, version, palette);
    let Some(counter) = vm.block_counter().filter(|_| !vm.is_empty()) else {
        return bar;
    };

    let widths = counter_widths(bar.width(), counter);

    match CounterParts::fit(available, widths) {
        CounterParts::WithKeys => {
            bar.left(counter_item(counter, counter.keys()))
        },
        CounterParts::TextOnly => bar.left(counter_item(counter, &[])),
        _ => bar,
    }
}

/// The status bar of `vm` with everything but the block counter.
fn bar_without_counter<Message: Clone>(
    vm: &StatusBarViewModel,
    version: &str,
    palette: &UiPalette,
) -> StatusBar<Message> {
    let mut bar = StatusBar::new(palette).left(StatusItem::plain_target(
        vm.machine_target(),
        vm.machine_name(),
    ));

    if !vm.is_empty() {
        if let Some(folder) = vm.folder() {
            bar = bar.left(StatusItem::mono(folder));
        }

        if let Some(count) = vm.pane_count() {
            bar = bar.right(StatusItem::text(i18n::status_panes(count)));
        }

        bar = bar
            .right(StatusItem::text(i18n::status_sessions(vm.session_count())));
    }

    bar.right(StatusItem::text(format!("OTTY {version}")))
}

/// The widths of a bar `bar_width` pixels wide without the block counter
/// once `counter` is added, in full and with its text only. The counter is
/// one more item at the end of the left side, after the gap between items.
fn counter_widths(bar_width: f32, counter: &BlockCounter) -> CounterWidths {
    let rest = bar_width + size::STATUS_GAP;
    let with_keys = counter_item::<()>(counter, counter.keys());
    let text_only = counter_item::<()>(counter, &[]);

    CounterWidths {
        with_keys: rest + with_keys.width(),
        text_only: rest + text_only.width(),
    }
}

/// The item of the block counter: `Block 5 of 6` and `keys`.
fn counter_item<Message>(
    counter: &BlockCounter,
    keys: &[String],
) -> StatusItem<Message> {
    let text = i18n::status_block_place(counter.number(), counter.count());

    StatusItem::with_keys(text, keys.iter().cloned())
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_domain::shortcuts::AppShortcut;
    use otty_app_theme::theme::ThemeManager;
    use otty_ui_kit::components::target_dot::TargetKind;

    use super::*;
    use crate::frame::status_bar::StatusBarInput;

    fn vm(
        folder: Option<&str>,
        pane_count: Option<usize>,
        session_count: usize,
    ) -> StatusBarViewModel {
        StatusBarViewModel::new(StatusBarInput {
            machine_name: "fedora",
            machine_target: TargetKind::Local,
            folder,
            block_place: None,
            pane_count,
            session_count,
        })
    }

    #[test]
    fn a_selected_block_shows_its_counter_with_the_two_keys() {
        let themes = ThemeManager::new();
        let vm = StatusBarViewModel::new(StatusBarInput {
            machine_name: "fedora",
            machine_target: TargetKind::Local,
            folder: Some("~/otty"),
            block_place: Some((5, 6)),
            pane_count: Some(1),
            session_count: 1,
        });
        let mut ui = simulator(view::<()>(
            &vm,
            "0.4.0",
            ThemeProps::new(themes.current()),
        ));

        for label in [
            i18n::status_block_place(5, 6),
            AppShortcut::PreviousBlock.label(),
            AppShortcut::NextBlock.label(),
        ] {
            ui.find(label.as_str())
                .expect("the counter should be visible");
        }
    }

    /// The status bar with block 5 of 6 selected.
    fn vm_with_counter() -> StatusBarViewModel {
        StatusBarViewModel::new(StatusBarInput {
            machine_name: "fedora",
            machine_target: TargetKind::Local,
            folder: Some("~/otty"),
            block_place: Some((5, 6)),
            pane_count: Some(1),
            session_count: 1,
        })
    }

    /// Which of the counter text and its two keys a status bar `width`
    /// wide shows.
    fn counter_shown_at(width: f32) -> [bool; 3] {
        let themes = ThemeManager::new();
        let vm = vm_with_counter();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(width, 24.0),
            view::<()>(&vm, "0.4.0", ThemeProps::new(themes.current())),
        );

        [
            i18n::status_block_place(5, 6),
            AppShortcut::PreviousBlock.label(),
            AppShortcut::NextBlock.label(),
        ]
        .map(|label| ui.find(label.as_str()).is_ok())
    }

    /// Owner, issue 11: in a narrow status bar the two keys are left out
    /// first, then `Block 5 of 6`.
    #[test]
    fn in_a_narrow_status_bar_the_keys_go_first_then_the_counter() {
        let themes = ThemeManager::new();
        let palette = ThemeProps::new(themes.current()).theme.ui().clone();
        let vm = vm_with_counter();
        let counter = vm.block_counter().expect("a block is selected");
        let bar = bar_without_counter::<()>(&vm, "0.4.0", &palette);
        let CounterWidths {
            with_keys,
            text_only,
        } = counter_widths(bar.width(), counter);

        assert_eq!(counter_shown_at(with_keys), [true, true, true]);
        assert_eq!(counter_shown_at(with_keys - 1.0), [true, false, false]);
        assert_eq!(counter_shown_at(text_only - 1.0), [false, false, false]);
    }

    /// The bar is measured once without the counter; the widths it gives
    /// with the counter are those of the bar built with it.
    #[test]
    fn the_widths_with_the_counter_are_those_of_the_bar_built_with_it() {
        let themes = ThemeManager::new();
        let palette = ThemeProps::new(themes.current()).theme.ui().clone();
        let vm = vm_with_counter();
        let counter = vm.block_counter().expect("a block is selected");
        let without = bar_without_counter::<()>(&vm, "0.4.0", &palette);

        let widths = counter_widths(without.width(), counter);

        let with_keys = bar_without_counter::<()>(&vm, "0.4.0", &palette)
            .left(counter_item(counter, counter.keys()))
            .width();
        let text_only = bar_without_counter::<()>(&vm, "0.4.0", &palette)
            .left(counter_item(counter, &[]))
            .width();
        assert!((widths.with_keys - with_keys).abs() < 0.01);
        assert!((widths.text_only - text_only).abs() < 0.01);
    }

    #[test]
    fn given_terminal_tab_when_rendered_then_machine_folder_counts_and_version_show()
     {
        let themes = ThemeManager::new();
        let vm = vm(Some("~/otty"), Some(2), 3);
        let mut ui = simulator(view::<()>(
            &vm,
            "0.4.0",
            ThemeProps::new(themes.current()),
        ));

        for label in ["fedora", "~/otty", "2 panes", "3 sessions", "OTTY 0.4.0"]
        {
            ui.find(label).expect("status bar item should be visible");
        }
    }

    #[test]
    fn given_no_tabs_when_rendered_then_machine_and_version_show() {
        let themes = ThemeManager::new();
        let vm = vm(None, None, 0);
        let mut ui = simulator(view::<()>(
            &vm,
            "0.4.0",
            ThemeProps::new(themes.current()),
        ));

        ui.find("fedora").expect("machine should be visible");
        ui.find("OTTY 0.4.0").expect("version should be visible");
        assert!(ui.find("0 sessions").is_err());
    }
}
