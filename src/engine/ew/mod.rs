use egui::{Order, Ui, WidgetText};

pub struct EguiDialogue {
    pub title: WidgetText,
    pub open: bool,
    pub contents: Box<dyn Fn(&mut Ui, &mut bool)>,
}


impl EguiDialogue {
    fn show(&mut self, ctx: &egui::Context) {
        let window = egui::Window::new(self.title.clone());

        window.order(Order::TOP)
            .show(ctx, |ui| {
                (self.contents)(ui, &mut self.open);
            });
    }
}
