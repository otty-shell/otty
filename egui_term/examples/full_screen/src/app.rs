use egui::Vec2;

pub struct App {
    terminal_backend: egui_term::otty::TerminalBackend,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let system_shell = std::env::var("SHELL")
            .expect("SHELL variable is not defined")
            .to_string();

        let terminal_backend = egui_term::otty::TerminalBackend::new(
            0,
            cc.egui_ctx.clone(),
            egui_term::BackendSettings {
                shell: system_shell,
                ..Default::default()
            },
        )
        .unwrap();

        Self { terminal_backend }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let terminal = egui_term::otty::TerminalView::new(
                ui,
                &mut self.terminal_backend,
            )
            .set_focus(true)
            .set_size(Vec2::new(ui.available_width(), ui.available_height()));

            ui.add(terminal);
        });
    }
}
