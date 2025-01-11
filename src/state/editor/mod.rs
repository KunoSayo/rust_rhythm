mod util;
mod editor;

use std::sync::Arc;
use crate::engine::{GameState, LoopState, StateData, Trans, WaitFutureState, WaitResult};
use crate::game::song::SongManager;
use egui::Context;
use crate::state::editor::editor::BeatMapEditor;

pub struct EditorMenu {}

impl EditorMenu {
    pub fn new() -> Self {
        Self {}
    }
}


impl GameState for EditorMenu {
    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        (Trans::None, LoopState::WAIT)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut tran = Trans::None;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.allocate_space((0.0, 100.0).into());

                if ui.button("Music | 音乐").clicked() {
                    if let Some(music) = util::select_music_file(&s.app.window) {
                        let song_manager = s.wd.world.get_mut::<Arc<SongManager>>()
                                .expect("How can we lost song manager")
                            .clone();

                        tran = Trans::Push(WaitFutureState::wait_task(async move {
                            let result = song_manager.import_song(&music);
                            match result {
                                Ok(e) => {
                                    let editor = BeatMapEditor::new(e);
                                    let editor = Box::new(editor);
                                    WaitResult::Push(editor)
                                }
                                Err(e) => {
                                    log::warn!("Failed to import music for {:?}", e);
                                    WaitResult::Function(Box::new(|_| {
                                        Trans::None
                                    }))
                                }
                            }
                        }));
                    }
                }
            })
        });

        tran
    }
}