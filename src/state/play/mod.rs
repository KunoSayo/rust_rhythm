mod gaming;

use crate::engine::{
    GameState, LoopState, StateData, StateEvent, Trans, WaitFutureState, WaitResult,
};
use crate::game::song::{SongManager, SongManagerResourceType};
use crate::state::play::gaming::GamingState;
use crate::ui::song_list::SongListUi;
use egui::{Align, Context, Frame, Layout, Pos2, Rect, UiBuilder, UiKind, UiStackInfo};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct PlayMenu {
    ui: SongListUi,
}

impl PlayMenu {
    pub fn new() -> Self {
        Self {
            ui: Default::default(),
        }
    }

    fn update_ui(&mut self, s: &mut StateData) {
        let songs =
            s.wd.world
                .get_mut::<SongManagerResourceType>()
                .unwrap()
                .songs
                .iter()
                .map(|x| x.value().clone())
                .collect::<Vec<_>>();
        if !songs.par_iter().any(|x| x.dirty.load(Ordering::Relaxed)) {
            self.ui.update_songs(songs);
        }
    }
}

impl GameState for PlayMenu {
    fn start(&mut self, s: &mut StateData) -> LoopState {
        self.update_ui(s);
        LoopState::WAIT
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        if self
            .ui
            .songs()
            .par_iter()
            .any(|x| x.dirty.load(Ordering::Relaxed))
        {
            self.update_ui(s);
        }
        let mut tran = Trans::None;
        if s.app
            .inputs
            .is_pressed(&[PhysicalKey::Code(KeyCode::Escape)])
        {
            tran = Trans::Pop;
        }
        (tran, LoopState::WAIT)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut tran = Trans::None;
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let height = ui.available_height();
                let width = ui.available_width() / 2.0;
                let left_rect = Rect {
                    min: Pos2::ZERO,
                    max: Pos2::new(width, height),
                };
                let builder = UiBuilder::new()
                    .max_rect(left_rect)
                    .ui_stack_info(UiStackInfo::new(UiKind::GenericArea));
                ui.allocate_new_ui(builder, |ui| {
                    ui.vertical(|ui| {
                        ui.allocate_space((0.0, 100.0).into());
                    });
                });

                let right_rect = Rect {
                    min: Pos2::new(width, 0.0),
                    max: Pos2::new(width * 2.0, height),
                };
                let builder = UiBuilder::new()
                    .max_rect(right_rect)
                    .ui_stack_info(UiStackInfo::new(UiKind::GenericArea))
                    .layout(Layout::top_down(Align::RIGHT));
                ui.allocate_new_ui(builder, |ui| {
                    let response = self.ui.ui(ui);
                    if let Some(result) = response.result {
                        if let Some(beatmap) = &result.beatmap {
                            let song_info = result.song;
                            let beatmap = beatmap.song_beatmap_file.clone();
                            let handle = s.app.audio.as_mut().unwrap().stream_handle.clone();
                            tran = Trans::Push(WaitFutureState::wait_task(async move {
                                let state = GamingState::new(handle, &song_info, beatmap);
                                match state {
                                    Ok(state) => {
                                        let state = Box::new(state);
                                        WaitResult::Push(state)
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to import music for {:?}", e);
                                        WaitResult::Function(Box::new(|_| Trans::None))
                                    }
                                }
                            }));
                        }
                    }
                });
            });

        tran
    }

    fn on_event(&mut self, s: &mut StateData, event: StateEvent) {
        match event {
            StateEvent::Resume => {
                s.app.window.set_title("Rust Rhythm");
            }
            _ => {}
        }
    }
}
