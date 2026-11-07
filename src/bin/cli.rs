use anyhow::{Result, ensure};
use clap::Parser;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use telysta::{
    BatchEvent, BookFormat, ConversionJob, ConversionOptions, Direction, convert_batch, discover,
};

#[derive(Parser)]
#[command(
    version,
    about = "Telysta · 将图片、文件夹与压缩包制作成 EPUB / PDF / CBZ"
)]
struct Args {
    /// 图片、文件夹或压缩包（可多选；一起选择的单张图片合为一本书）
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// 输出格式
    #[arg(short, long, value_enum, default_value = "epub")]
    format: BookFormat,
    /// 输出父目录；自动新建 Telysta Books 子文件夹
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// 书名（仅适用于一本书）
    #[arg(long)]
    title: Option<String>,
    /// 作者
    #[arg(long, default_value = "")]
    author: String,
    #[arg(long, value_enum, default_value = "ltr")]
    direction: Direction,
    /// 需要转为 JPEG 时的质量（1–100）；原始 JPEG/PNG 不重复编码
    #[arg(long, default_value_t = 90, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: u8,
    /// 同时制作的书籍数量（1–8）；每本书内部仍按编号顺序写入
    #[arg(short='j',long,default_value_t=telysta::batch::DEFAULT_JOBS as u8,value_parser=clap::value_parser!(u8).range(1..=8))]
    jobs: u8,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let sources = discover(args.inputs)?;
    ensure!(!sources.is_empty(), "没有可转换的文件");
    ensure!(
        args.title.is_none() || sources.len() == 1,
        "批量制作时请保留原文件名，不要指定 --title"
    );
    let mut jobs = Vec::new();
    for source in &sources {
        let mut options = ConversionOptions::for_source(source);
        options.format = args.format;
        options.author.clone_from(&args.author);
        options.direction = args.direction;
        options.jpeg_quality = args.quality;
        if let Some(parent) = &args.output {
            options.output_parent.clone_from(parent);
        }
        if let Some(title) = &args.title {
            options.title.clone_from(title);
        }
        jobs.push(ConversionJob {
            source: source.clone(),
            options,
        });
    }
    let titles = jobs
        .iter()
        .map(|job| job.options.title.clone())
        .collect::<Vec<_>>();
    let failures = AtomicUsize::new(0);
    let milestones = (0..jobs.len())
        .map(|_| AtomicUsize::new(usize::MAX))
        .collect::<Vec<_>>();
    eprintln!("准备制作 {} 本书籍，最多 {} 本并发", jobs.len(), args.jobs);
    convert_batch(
        jobs,
        usize::from(args.jobs),
        &AtomicBool::new(false),
        |event| match event {
            BatchEvent::Progress { index, progress } => {
                if progress.pages_total > 0
                    && (progress.pages_done == progress.pages_total
                        || milestones[index].swap(progress.pages_done / 20, Ordering::Relaxed)
                            != progress.pages_done / 20)
                {
                    eprintln!("[{}] {}", titles[index], progress.message);
                }
            }
            BatchEvent::Finished { index, result } => match result {
                Ok(result) => println!(
                    "{} · {} 页 · {:.1} MiB",
                    result.path.display(),
                    result.pages,
                    result.bytes as f64 / 1_048_576.0
                ),
                Err(error) => {
                    failures.fetch_add(1, Ordering::Relaxed);
                    eprintln!("[{}] {error:#}", titles[index]);
                }
            },
        },
    )?;
    let failures = failures.load(Ordering::Relaxed);
    ensure!(
        failures == 0,
        "{} 本成功，{failures} 本失败",
        sources.len() - failures
    );
    Ok(())
}
