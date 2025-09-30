use std::sync::{Arc, Mutex};
use std::time::Duration;

use atty::Stream as AttyStream;
use console::{Color, style};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

use crate::screenshotter::args::BrowserKind;
use crate::screenshotter::models::{CaseResult, CaseStatus, MismatchSeverity};

#[derive(Clone)]
pub struct Logger {
    inner: Arc<LoggerInner>,
}

struct LoggerInner {
    is_tty: bool,
    stdout: Mutex<()>,
    stderr: Mutex<()>,
}

pub struct ProgressGroup {
    _multi: Arc<MultiProgress>,
    capture: ProgressBar,
    compare: ProgressBar,
}

impl ProgressGroup {
    pub fn capture(&self) -> &ProgressBar {
        &self.capture
    }

    pub fn compare(&self) -> &ProgressBar {
        &self.compare
    }

    pub fn finish_capture(&self) {
        self.capture.finish_and_clear();
    }
}

#[derive(Copy, Clone)]
pub enum WarnLevel {
    Low,
    Medium,
    High,
}

#[derive(Copy, Clone)]
enum LogLevel {
    Info,
    Success,
    Warn(WarnLevel),
    Error,
    Detail,
}

#[derive(Copy, Clone)]
enum LogTarget {
    Stdout,
    Stderr,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(LoggerInner {
                is_tty: atty::is(AttyStream::Stdout),
                stdout: Mutex::new(()),
                stderr: Mutex::new(()),
            }),
        }
    }

    pub fn is_tty(&self) -> bool {
        self.inner.is_tty
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log(None, LogLevel::Info, message.into());
    }

    pub fn success(&self, message: impl Into<String>) {
        self.log(None, LogLevel::Success, message.into());
    }

    pub fn warn(&self, message: impl Into<String>) {
        self.log(None, LogLevel::Warn(WarnLevel::Medium), message.into());
    }

    pub fn warn_with_progress(
        &self,
        pb: Option<&ProgressBar>,
        level: WarnLevel,
        message: impl Into<String>,
    ) {
        self.log(pb, LogLevel::Warn(level), message.into());
    }

    pub fn error(&self, message: impl Into<String>) {
        self.log(None, LogLevel::Error, message.into());
    }

    pub fn detail(&self, pb: Option<&ProgressBar>, message: impl Into<String>) {
        self.log(pb, LogLevel::Detail, message.into());
    }

    pub fn blank(&self) {
        println!();
    }

    pub fn browser_banner(&self, browser: BrowserKind, total_cases: usize) {
        let text = style(format!("{browser} • {total_cases} cases"))
            .cyan()
            .bold();
        self.log(None, LogLevel::Info, text.to_string());
    }

    pub fn progress_group(&self, total: usize, browser: BrowserKind) -> Option<ProgressGroup> {
        if !self.is_tty() {
            return None;
        }

        let draw_target = ProgressDrawTarget::stderr_with_hz(20);
        let multi = Arc::new(MultiProgress::with_draw_target(draw_target));
        let base_style = ProgressStyle::with_template(
            "{prefix} {wide_bar} {pos}/{len} [{elapsed_precise}<{eta_precise}] {msg}",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏  ");

        let capture = multi.add(ProgressBar::new(total as u64));
        capture.set_style(base_style.clone());
        capture.set_prefix(format!(
            "{} {}",
            style(browser.to_string()).cyan().bold(),
            style("🎨").dim()
        ));
        capture.enable_steady_tick(Duration::from_millis(150));

        let compare = multi.add(ProgressBar::new(total as u64));
        compare.set_style(base_style);
        compare.set_prefix(format!(
            "{} {}",
            style(browser.to_string()).cyan().bold(),
            style("🔍").dim()
        ));
        compare.enable_steady_tick(Duration::from_millis(150));

        Some(ProgressGroup {
            _multi: multi,
            capture,
            compare,
        })
    }

    pub fn case_intro(
        &self,
        pb: Option<&ProgressBar>,
        _index: usize,
        _total: usize,
        key: &str,
        _browser: BrowserKind,
    ) {
        let message = key.to_string();
        if let Some(pb) = pb {
            pb.set_message(message);
        } else {
            self.log(None, LogLevel::Detail, message);
        }
    }

    pub fn case_pass(
        &self,
        pb: Option<&ProgressBar>,
        key: &str,
        _browser: BrowserKind,
        duration_ms: Option<f64>,
    ) {
        let timing = duration_ms
            .map(|ms| format!("– {:.1}ms", ms))
            .unwrap_or_default();
        let message = format!("{key} {timing}");
        if let Some(pb) = pb {
            pb.inc(1);
            pb.set_message(self.render_line(LogLevel::Success, message));
        } else {
            self.log(None, LogLevel::Success, message);
        }
    }

    pub fn case_failure(
        &self,
        pb: Option<&ProgressBar>,
        status: CaseStatus,
        key: &str,
        browser: BrowserKind,
        message: String,
    ) {
        let full_message = format!("{key} ({browser}) {:?}: {message}", status);
        let rendered = self.render_line(LogLevel::Error, full_message.clone());
        if let Some(pb) = pb {
            pb.inc(1);
            pb.set_message(rendered.clone());
            pb.println(rendered);
        } else {
            self.log(None, LogLevel::Error, full_message);
        }
    }

    pub fn case_mismatch(
        &self,
        pb: Option<&ProgressBar>,
        key: &str,
        browser: BrowserKind,
        severity: MismatchSeverity,
        message: String,
    ) {
        let warn_level = warn_level_for_mismatch(severity);

        let indicator = match severity {
            MismatchSeverity::Minor => "△",
            MismatchSeverity::Noticeable => "▲",
            MismatchSeverity::Major => "✖",
        };

        if let Some(pb) = pb {
            pb.inc(1);
            let rendered = self.render_line(
                LogLevel::Warn(warn_level),
                format!("{key} ({browser}) {message}"),
            );
            pb.set_message(rendered.clone());
            pb.println(rendered);
            pb.set_message(format!("{indicator} {key} ({browser})"));
        } else {
            self.log(
                None,
                LogLevel::Warn(warn_level),
                format!("{key} ({browser}) mismatch: {message}"),
            );
        }
    }

    pub fn retrying(&self, pb: Option<&ProgressBar>, message: impl Into<String>) {
        let text = message.into();
        if let Some(pb) = pb {
            pb.println(self.render_line(LogLevel::Detail, text.clone()));
        } else {
            self.log(None, LogLevel::Detail, text);
        }
    }

    pub fn finish_progress(&self, pb: Option<ProgressBar>, message: impl Into<String>) {
        if let Some(pb) = pb {
            pb.finish_with_message(message.into());
        }
    }

    fn log(&self, pb: Option<&ProgressBar>, level: LogLevel, message: String) {
        let rendered = self.render_line(level, message);
        let target = log_target(level);

        if let Some(pb) = pb {
            let inner = Arc::clone(&self.inner);
            let rendered_clone = rendered.clone();
            pb.suspend(move || {
                let lock = match target {
                    LogTarget::Stdout => inner.stdout.lock().unwrap(),
                    LogTarget::Stderr => inner.stderr.lock().unwrap(),
                };
                drop(lock);

                match target {
                    LogTarget::Stdout => println!("{rendered_clone}"),
                    LogTarget::Stderr => eprintln!("{rendered_clone}"),
                }
            });
            return;
        }

        let lock = match target {
            LogTarget::Stdout => self.inner.stdout.lock().unwrap(),
            LogTarget::Stderr => self.inner.stderr.lock().unwrap(),
        };
        drop(lock);

        match target {
            LogTarget::Stdout => println!("{rendered}"),
            LogTarget::Stderr => eprintln!("{rendered}"),
        }
    }

    fn render_line(&self, level: LogLevel, message: String) -> String {
        let (icon, styled_msg) = match level {
            LogLevel::Info => (style("•").cyan(), style(message)),
            LogLevel::Success => (style("✔").green().bold(), style(message).green().bold()),
            LogLevel::Warn(warn) => {
                let (color, icon) = match warn {
                    WarnLevel::Low => (Color::Color256(220), "△"),
                    WarnLevel::Medium => (Color::Color256(208), "▲"),
                    WarnLevel::High => (Color::Color256(196), "▲"),
                };
                (style(icon).fg(color).bold(), style(message).fg(color))
            }
            LogLevel::Error => (style("✖").red().bold(), style(message).red()),
            LogLevel::Detail => (style("↻").dim(), style(message).dim()),
        };
        format!("{} {}", icon, styled_msg)
    }
}

fn log_target(level: LogLevel) -> LogTarget {
    match level {
        LogLevel::Warn(_) | LogLevel::Error => LogTarget::Stderr,
        LogLevel::Info | LogLevel::Success | LogLevel::Detail => LogTarget::Stdout,
    }
}

fn warn_level_for_mismatch(severity: MismatchSeverity) -> WarnLevel {
    match severity {
        MismatchSeverity::Minor => WarnLevel::Low,
        MismatchSeverity::Noticeable => WarnLevel::Medium,
        MismatchSeverity::Major => WarnLevel::High,
    }
}

fn max_warn_level(a: WarnLevel, b: WarnLevel) -> WarnLevel {
    match (a, b) {
        (WarnLevel::High, _) | (_, WarnLevel::High) => WarnLevel::High,
        (WarnLevel::Medium, _) | (_, WarnLevel::Medium) => WarnLevel::Medium,
        _ => WarnLevel::Low,
    }
}

pub fn summarize_failures(logger: &Logger, failures: &[(String, CaseResult)]) -> Option<WarnLevel> {
    if failures.is_empty() {
        return None;
    }

    let mut highest: Option<WarnLevel> = None;

    for (_, failure) in failures {
        let level = match failure.status {
            CaseStatus::Error => WarnLevel::High,
            CaseStatus::Mismatch => failure
                .severity
                .map(warn_level_for_mismatch)
                .unwrap_or(WarnLevel::High),
            CaseStatus::Pass => continue,
        };

        highest = Some(match highest {
            Some(current) => max_warn_level(current, level),
            None => level,
        });
    }

    if let Some(level) = highest {
        match level {
            WarnLevel::High => logger.error("Failure summary:"),
            other => logger.warn_with_progress(None, other, "Failure summary:"),
        };

        for (name, failure) in failures {
            let message = failure
                .message
                .clone()
                .unwrap_or_else(|| "<no error message>".to_string());
            match failure.status {
                CaseStatus::Error => {
                    logger.error(format!("\"{message}\" for {name}"));
                }
                CaseStatus::Mismatch => {
                    let warn_level = failure
                        .severity
                        .map(warn_level_for_mismatch)
                        .unwrap_or(WarnLevel::High);
                    logger.warn_with_progress(
                        None,
                        warn_level,
                        format!("\"{message}\" for {name}"),
                    );
                }
                CaseStatus::Pass => {}
            }
        }

        Some(level)
    } else {
        None
    }
}
