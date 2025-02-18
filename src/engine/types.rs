use egui::{Button, Response, Ui, Vec2, WidgetText};

pub trait EguiExt {
    #[inline]
    fn get_ui(&mut self) -> &mut Ui;

    fn select_button(&mut self, text: impl Into<WidgetText>, selected: bool, size: Vec2) -> Response {
        let button = Button::new(text)
            .selected(selected)
            .min_size(size);
        self.get_ui().add(button)
    }
}

impl EguiExt for Ui {
    fn get_ui(&mut self) -> &mut Ui {
        self
    }
}