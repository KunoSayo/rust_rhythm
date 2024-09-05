use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

#[derive(Default)]
struct CounterInner {
    loading: AtomicU16,
    finished: AtomicU16,
    errors: AtomicU16,
}

#[derive(Default, Clone)]
pub struct CounterProgress {
    inner: Arc<CounterInner>,
}


pub struct CounterProgressTracker {
    loaded: bool,
    inner: Arc<CounterInner>,
}

pub trait Progress {
    type Tracker: ProgressTracker;
    fn num_loading(&self) -> u16;

    fn num_finished(&self) -> u16;

    fn error_nums(&self) -> u16;
    fn create_tracker(&self) -> Self::Tracker;
}

pub trait ProgressTracker: 'static + Send {
    fn end_loading(&mut self) {}

    fn new_error_num(&mut self) {}
}

impl Progress for CounterProgress {
    type Tracker = CounterProgressTracker;

    fn num_loading(&self) -> u16 {
        self.inner.loading.load(Ordering::Acquire)
    }

    fn num_finished(&self) -> u16 {
        self.inner.finished.load(Ordering::Acquire)
    }

    fn error_nums(&self) -> u16 {
        self.inner.errors.load(Ordering::Acquire)
    }

    fn create_tracker(&self) -> Self::Tracker {
        self.inner.loading.fetch_add(1, Ordering::AcqRel);
        CounterProgressTracker {
            loaded: false,
            inner: self.inner.clone(),
        }
    }
}

impl Progress for () {
    type Tracker = ();

    fn num_loading(&self) -> u16 {
        0
    }

    fn num_finished(&self) -> u16 {
        0
    }

    fn error_nums(&self) -> u16 {
        0
    }

    fn create_tracker(&self) -> Self::Tracker {
        ()
    }
}

impl ProgressTracker for () {}

impl ProgressTracker for CounterProgressTracker {
    fn end_loading(&mut self) {
        self.loaded = true;
        self.inner.loading.fetch_sub(1, Ordering::AcqRel);
        self.inner.finished.fetch_add(1, Ordering::AcqRel);
    }

    fn new_error_num(&mut self) {
        if !self.loaded {
            self.end_loading();
        }
        self.inner.errors.fetch_add(1, Ordering::AcqRel);
    }
}

impl Drop for CounterProgressTracker {
    fn drop(&mut self) {
        if !self.loaded {
            //now loaded.
            self.end_loading();
        }
    }
}
