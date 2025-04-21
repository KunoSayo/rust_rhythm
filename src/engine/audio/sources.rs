use num::CheckedMul;
use rodio::buffer::SamplesBuffer;
use rodio::source::{SeekError, TrackPosition};
use rodio::{Sample, Source};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::Duration;
use crate::engine::OutputStreamHandle;

enum ControlEvent {
    SetVol(f32),
    Seek(Duration),
    Play,
    Stop,
}

#[derive(Clone, Default)]
struct SharedMem {
    duration: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
}

pub const DELAY_MS_ALLOW: u32 = 4;

struct ControlledSampleBuffers {
    buffer: TrackPosition<SamplesBuffer>,
    update_left: u128,
    update_freq: u128,
    vol: f32,
    stop: bool,
    pause: bool,
    /// The duration in ms
    shared: SharedMem,
    rx: Receiver<ControlEvent>,
}

impl ControlledSampleBuffers {
    fn new(
        buffer: SamplesBuffer,
        update_dur: Duration,
        shared: SharedMem,
        rx: Receiver<ControlEvent>,
    ) -> Self {
        let micros = update_dur.as_micros();
        //
        // (micros / ..) = s
        // s * sample/s = sample
        let update_freq = micros
            .checked_mul(buffer.channels() as u128)
            .unwrap()
            .checked_mul(buffer.sample_rate() as u128)
            .unwrap()
            / 1_000_000;
        let update_freq = update_freq.max(1);
        Self {
            buffer: buffer.track_position(),
            vol: 1.0,
            stop: false,
            pause: true,
            shared,
            update_left: 1,
            update_freq,
            rx,
        }
    }

    fn update_info(&mut self) {
        self.shared
            .duration
            .store(self.buffer.get_pos().as_micros() as u64, Ordering::Relaxed);
        loop {
            match self.rx.try_recv() {
                Ok(event) => match event {
                    ControlEvent::SetVol(vol) => {
                        self.vol = vol;
                    }
                    ControlEvent::Seek(d) => self.buffer.try_seek(d).unwrap(),
                    ControlEvent::Stop => {
                        self.stop = true;
                    }
                    ControlEvent::Play => {
                        self.pause = false;
                    }
                },
                Err(TryRecvError::Disconnected) => {
                    self.stop = true;
                    self.shared.stopped.store(true, Ordering::Relaxed);
                }
                Err(TryRecvError::Empty) => break,
            }
        }
    }
}

impl Iterator for ControlledSampleBuffers {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop {
            return None;
        }
        if self.pause {
            self.update_info();
            if self.pause {
                return Some(0.0);
            }
        }
        self.update_left -= 1;
        if self.update_left == 0 {
            self.update_left = self.update_freq;
            // the engine poll multi buffers together.
            // println!("Update info when {:?}", std::time::Instant::now());
            self.update_info();
        }
        self.buffer.next().map(|x| x * self.vol)
    }
}

impl Source for ControlledSampleBuffers {
    fn current_span_len(&self) -> Option<usize> {
        // we update per ms
        Some((DELAY_MS_ALLOW * self.buffer.sample_rate() * self.buffer.channels() as u32 / 1000).max(1) as usize)
    }

    fn channels(&self) -> u16 {
        self.buffer.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.buffer.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.buffer.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.buffer.try_seek(pos)
    }
}

impl Drop for ControlledSampleBuffers {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Relaxed);
    }
}

pub struct ControlledBufferHandle {
    tx: Sender<ControlEvent>,
    mem: SharedMem,
    vol: f32,
}

impl ControlledBufferHandle {
    pub fn new(output: &OutputStreamHandle, buffer: SamplesBuffer) -> anyhow::Result<Self> {
        let mem = SharedMem::default();
        let (tx, rx) = channel();
        let source =
            ControlledSampleBuffers::new(buffer, Duration::from_millis(DELAY_MS_ALLOW as u64), mem.clone(), rx);
        output.add(source);
        let this = Self { tx, mem, vol: 1.0 };
        Ok(this)
    }

    pub fn volume(&self) -> f32 {
        self.vol
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.vol = vol;
        let _ = self.tx.send(ControlEvent::SetVol(vol));
    }

    pub fn get_micros(&self) -> u64 {
        self.mem.duration.load(Ordering::Relaxed)
    }

    pub fn seek_to(&self, d: Duration) {
        let _ = self.tx.send(ControlEvent::Seek(d));
    }

    pub fn is_stopped(&self) -> bool {
        self.mem.stopped.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        let _ = self.tx.send(ControlEvent::Stop);
    }

    pub fn play(&self) {
        let _ = self.tx.send(ControlEvent::Play);
    }
}
