mod util;
mod editor;
mod timing_editor;

use crate::engine::{GameState, LoopState, StateData, Trans, WaitFutureState, WaitResult};
use crate::game::song::{SongManager, SongManagerResourceType};
use crate::state::editor::editor::BeatMapEditor;
use crate::ui::song_list::{EnterResult, SongListUi};
use egui::{Align, Color32, Context, Frame, Layout, Pos2, Rect, UiBuilder, UiKind, UiStackInfo, Widget};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct EditorMenu {
    ui: SongListUi,
}

impl EditorMenu {
    pub fn new() -> Self {
        Self { ui: Default::default() }
    }

    fn update_ui(&mut self, s: &mut StateData) {
        let songs = s.wd.world.get_mut::<SongManagerResourceType>().unwrap().songs.iter()
            .map(|x| x.value().clone())
            .collect::<Vec<_>>();
        if !songs.par_iter().any(|x| x.dirty.load(Ordering::Relaxed)) {
            self.ui.update_songs(songs);
        }
    }
}


impl GameState for EditorMenu {
    fn start(&mut self, s: &mut StateData) {
        self.update_ui(s);
    }

    fn update(&mut self, s: &mut StateData) -> (Trans, LoopState) {
        if self.ui.songs().par_iter().any(|x| x.dirty.load(Ordering::Relaxed)) {
            self.update_ui(s);
        }
        let mut tran = Trans::None;
        if s.app.inputs.is_pressed(&[PhysicalKey::Code(KeyCode::Escape)]) {
            tran = Trans::Pop;
        }
        (tran, LoopState::WAIT)
    }

    fn render(&mut self, s: &mut StateData, ctx: &Context) -> Trans {
        let mut tran = Trans::None;
        egui::CentralPanel::default()
            .frame(Frame::none())
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

                        if ui.button("Music | 音乐").clicked() {
                            if let Some(music) = util::select_music_file(&s.app.window) {
                                let song_manager = s.wd.world.get_mut::<Arc<SongManager>>()
                                    .expect("How can we lost song manager")
                                    .clone();

                                let handle = s.app.audio.as_ref().unwrap().stream_handle.clone();
                                tran = Trans::Push(WaitFutureState::wait_task(async move {
                                    let result = song_manager.import_song(&music);
                                    match result {
                                        Ok(e) => {
                                            let editor = BeatMapEditor::new(e, handle);
                                            match editor {
                                                Ok(editor) => {
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
                        let editor = BeatMapEditor::with_file(result.song, result.beatmap, s.app.audio.as_ref().unwrap().stream_handle.clone());
                        match editor {
                            Ok(editor) => {
                                let editor = Box::new(editor);
                                tran = Trans::Push(editor);
                            }
                            Err(e) => {
                                panic!("Failed to open music, maybe error box soon? {}", e);
                            }
                        }
                    }
                });
            });

        tran
    }
}