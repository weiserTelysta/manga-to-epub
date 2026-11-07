use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};
use telysta::{BatchEvent, BookSource, ConversionJob, ConversionOptions, convert_batch};

fn jobs(root: &Path, count: usize, same_title: bool) -> Vec<ConversionJob> {
    (0..count)
        .map(|index| {
            let path = root.join(format!("{index:04}.png"));
            image::RgbImage::from_pixel(8, 12, image::Rgb([index as u8, 120, 200]))
                .save(&path)
                .unwrap();
            let source = BookSource::Images(vec![path]);
            let mut options = ConversionOptions::for_source(&source);
            options.title = if same_title {
                "同名漫画".into()
            } else {
                format!("漫画 {index}")
            };
            ConversionJob { source, options }
        })
        .collect()
}

#[test]
fn workers_run_in_parallel_with_a_fixed_limit() {
    let dir = tempfile::tempdir().unwrap();
    let (active, maximum, completed) = (
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicUsize::new(0),
    );
    convert_batch(
        jobs(dir.path(), 6, false),
        2,
        &AtomicBool::new(false),
        |event| match event {
            BatchEvent::Progress { progress, .. } if progress.fraction == 0.0 => {
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                maximum.fetch_max(count, Ordering::SeqCst);
                // Keep the real jobs overlapping long enough to observe the limit.
                std::thread::sleep(Duration::from_millis(25));
            }
            BatchEvent::Finished { result, .. } => {
                assert!(result.is_ok());
                active.fetch_sub(1, Ordering::SeqCst);
                completed.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        },
    )
    .unwrap();
    assert_eq!(maximum.load(Ordering::SeqCst), 2);
    assert_eq!(completed.load(Ordering::SeqCst), 6);
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[test]
fn parallel_duplicate_titles_do_not_overwrite_each_other() {
    let dir = tempfile::tempdir().unwrap();
    let completed = AtomicUsize::new(0);
    convert_batch(
        jobs(dir.path(), 4, true),
        4,
        &AtomicBool::new(false),
        |event| {
            if let BatchEvent::Finished { result, .. } = event {
                assert!(result.is_ok());
                completed.fetch_add(1, Ordering::Relaxed);
            }
        },
    )
    .unwrap();
    assert_eq!(completed.load(Ordering::Relaxed), 4);
    let outputs = fs::read_dir(dir.path().join("Telysta Books"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(outputs.len(), 4);
    for output in outputs {
        assert!(zip::ZipArchive::new(fs::File::open(output.path()).unwrap()).is_ok());
    }
}

#[test]
fn one_failed_book_does_not_stop_other_books() {
    let dir = tempfile::tempdir().unwrap();
    let tasks = jobs(dir.path(), 3, false);
    fs::write(tasks[1].source.path(), b"broken image").unwrap();
    let (success, failed) = (AtomicUsize::new(0), AtomicUsize::new(0));
    convert_batch(tasks, 2, &AtomicBool::new(false), |event| {
        if let BatchEvent::Finished { result, .. } = event {
            if result.is_ok() {
                success.fetch_add(1, Ordering::Relaxed);
            } else {
                failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    })
    .unwrap();
    assert_eq!(success.load(Ordering::Relaxed), 2);
    assert_eq!(failed.load(Ordering::Relaxed), 1);
    assert_eq!(
        fs::read_dir(dir.path().join("Telysta Books"))
            .unwrap()
            .count(),
        2
    );
}

#[test]
fn cancelled_batch_never_starts_queued_books() {
    let dir = tempfile::tempdir().unwrap();
    let calls = AtomicUsize::new(0);
    convert_batch(
        jobs(dir.path(), 3, false),
        2,
        &AtomicBool::new(true),
        |_| {
            calls.fetch_add(1, Ordering::Relaxed);
        },
    )
    .unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert!(!dir.path().join("Telysta Books").exists());
    assert!(convert_batch(Vec::new(), 0, &AtomicBool::new(false), |_| {}).is_err());
}
