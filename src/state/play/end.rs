use crate::engine::{GameState, LoopState, StateData, Trans};
use crate::game::beatmap::play::{Gaming, NoteResult};
use crate::game::beatmap::summary::{BeatmapPlayResult, HitSummary};
use egui::{
    Align, Color32, Context, Frame, Label, Layout, Pos2, Rect, RichText, Stroke, TextWrapMode,
    UiBuilder,
};
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct EndResultState {
    pub result: BeatmapPlayResult,
    pub gaming: Box<Gaming>,
}

impl GameState for EndResultState {
    fn start(&mut self, s: &mut StateData) -> LoopState {
        LoopState::WAIT
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        let mut trans = Trans::None;
        let cur_input = &s.app.inputs.cur_frame_input;
        if cur_input
            .pressing
            .contains(&PhysicalKey::Code(KeyCode::Escape))
        {
            trans = Trans::Pop;
        }
        (trans, LoopState::WAIT)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut trans = Trans::None;

        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let raw_height = ui.available_height();
                ui.vertical_centered(|ui| {
                    ui.heading(
                        RichText::new(&self.gaming.raw_file.metadata.title)
                            .strong()
                            .size(99.0),
                    );
                    ui.heading(
                        RichText::new(self.result.score.to_string())
                            .strong()
                            .size(50.0),
                    );
                });

                ui.vertical(|ui| {
                    macro_rules! label_result {
                        ($prefix: literal, $name: ident) => {
                            ui.label(
                                RichText::new(format!(
                                    "{}: {}",
                                    $prefix,
                                    self.gaming.score_counter.get_note_count(NoteResult::$name)
                                ))
                                .strong(),
                            );
                        };
                    }
                    label_result!("Perfect", Perfect);
                    label_result!("Great", Great);
                    label_result!("Good", Good);
                    label_result!("Bad", Bad);
                    label_result!("Miss", Miss);

                    ui.label(
                        RichText::new(format!(
                            "MaxCombo: {}",
                            self.gaming.score_counter.get_max_combo()
                        ))
                        .strong(),
                    );

                    let bottom_graph_rect = {
                        let bottom_graph_rect = ui.available_rect_before_wrap();
                        if bottom_graph_rect.height() * 2.0 >= raw_height {
                            Rect::from_min_max(
                                Pos2::new(
                                    bottom_graph_rect.left(),
                                    bottom_graph_rect.bottom() - raw_height * 0.5,
                                ),
                                bottom_graph_rect.right_bottom(),
                            )
                        } else {
                            bottom_graph_rect
                        }
                    };
                    debug_assert!(!bottom_graph_rect.any_nan());
                    let cells = self.result.hit_summary.delay_count.len() as f32;
                    let cell_width = (bottom_graph_rect.width() - 100.0) / cells;

                    let graph_top = bottom_graph_rect.top() - 100.0;

                    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                        let start_offset = 50.0 - cell_width / 2.0;

                        // the base line.
                        let bar_base_y = bottom_graph_rect.bottom() - 50.0;
                        ui.painter().hline(
                            bottom_graph_rect.left() + 50.0..=bottom_graph_rect.right() - 50.0,
                            bar_base_y,
                            Stroke::new(3.0, Color32::WHITE),
                        );

                        let bar_height = bar_base_y - graph_top;
                        for (idx, number) in HitSummary::bottom_numbers().enumerate() {
                            let text_rect = Rect::from_min_max(
                                Pos2::new(
                                    start_offset + idx as f32 * cell_width,
                                    bottom_graph_rect.bottom() - 50.0,
                                ),
                                Pos2::new(
                                    start_offset + idx as f32 * cell_width + cell_width,
                                    bottom_graph_rect.bottom(),
                                ),
                            );
                            let mid = 50.0 + idx as f32 * cell_width;

                            ui.painter().vline(
                                mid,
                                bottom_graph_rect.bottom() - 60.0
                                    ..=bottom_graph_rect.bottom() - 50.0,
                                Stroke::new(1.0, Color32::WHITE),
                            );
                            if idx < self.result.hit_summary.delay_count.len() {
                                let cur_cnt = self.result.hit_summary.delay_count[idx];
                                let current_height =
                                    cur_cnt as f32 * bar_height / self.result.hit_summary.mx as f32;
                                let bar_rect = Rect::from_min_max(
                                    Pos2::new(mid + 1.0, bar_base_y - current_height),
                                    Pos2::new(mid - 1.0 + cell_width, bar_base_y),
                                );
                                ui.painter().rect_filled(bar_rect, 0.0, Color32::DARK_GREEN);

                                let bar_text_rect = Rect::from_min_max(
                                    Pos2::new(bar_rect.left(), bar_rect.top() - 50.0),
                                    bar_rect.right_top(),
                                );
                                // the count text.
                                ui.allocate_new_ui(
                                    UiBuilder::new().max_rect(bar_text_rect),
                                    |ui| {
                                        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                                            ui.add_sized(
                                                ui.available_size(),
                                                Label::new(
                                                    RichText::new(cur_cnt.to_string())
                                                        .small()
                                                        .size(ui.available_width() / 2.5),
                                                )
                                                .wrap_mode(TextWrapMode::Extend)
                                                .selectable(false),
                                            );
                                        })
                                    },
                                );
                            }

                            // < 0 require 0
                            // > 0 require 1
                            if idx & 1 == if number < 0 { 0 } else { 1 } {
                                ui.allocate_new_ui(UiBuilder::new().max_rect(text_rect), |ui| {
                                    ui.centered_and_justified(|ui| {
                                        ui.add_sized(
                                            ui.available_size(),
                                            Label::new(
                                                RichText::new(number.to_string())
                                                    .small()
                                                    .size(ui.available_width() / 2.5),
                                            )
                                            .wrap_mode(TextWrapMode::Extend)
                                            .selectable(false),
                                        );
                                    })
                                });
                            }
                        }
                    });
                });
            });
        trans
    }
}
