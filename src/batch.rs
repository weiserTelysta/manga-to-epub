//! Bounded, per-book parallel conversion shared by the desktop and CLI.
use crate::{BookSource, ConversionOptions, ConversionResult, Progress, convert};
use anyhow::{Result, anyhow, ensure};
use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

pub const DEFAULT_JOBS: usize = 2;
pub const MAX_JOBS: usize = 8;

#[derive(Clone, Debug)]
pub struct ConversionJob {
    pub source: BookSource,
    pub options: ConversionOptions,
}

#[derive(Debug)]
pub enum BatchEvent {
    Progress {
        index: usize,
        progress: Progress,
    },
    Finished {
        index: usize,
        result: Result<ConversionResult>,
    },
}

/// Runs at most `parallelism` books at once. Each book preserves its own page
/// order, atomic output and cancellation checks. One failed book does not stop
/// the rest. Returns after all running workers have finished and cleaned up.
/// The callback may be invoked simultaneously by different workers.
pub fn convert_batch(
    jobs: Vec<ConversionJob>,
    parallelism: usize,
    cancel: &AtomicBool,
    emit: impl Fn(BatchEvent) + Sync,
) -> Result<()> {
    ensure!(
        (1..=MAX_JOBS).contains(&parallelism),
        "并发数量必须在 1 到 {MAX_JOBS} 之间"
    );
    let workers = parallelism.min(jobs.len());
    let queue = Mutex::new(jobs.into_iter().enumerate().collect::<VecDeque<_>>());
    thread::scope(|scope| {
        for _ in 0..workers {
            let (queue, emit) = (&queue, &emit);
            scope.spawn(move || {
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let job = queue.lock().expect("conversion queue poisoned").pop_front();
                    let Some((index, job)) = job else {
                        break;
                    };
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        convert(&job.source, &job.options, cancel, |progress| {
                            emit(BatchEvent::Progress { index, progress })
                        })
                    }))
                    .unwrap_or_else(|_| Err(anyhow!("转换遇到内部错误，请检查该文件")));
                    emit(BatchEvent::Finished { index, result });
                }
            });
        }
    });
    Ok(())
}
