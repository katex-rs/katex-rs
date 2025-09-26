use std::sync::Arc;
use std::time::Instant;

use image::RgbaImage;

use serde_json::Value as JsonValue;

use crate::screenshotter::args::BrowserKind;

#[derive(Clone, Debug)]
pub struct TestCase {
    pub key: String,
    pub payload: JsonValue,
}

#[derive(Clone, Debug)]
pub struct Screenshot {
    pub png: Vec<u8>,
    pub image: RgbaImage,
}

#[derive(Clone, Debug)]
pub struct BaselineEntry {
    pub image: Arc<RgbaImage>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CaseStatus {
    Pass,
    Mismatch,
    Error,
}

#[derive(Clone, Debug)]
pub struct CaseResult {
    pub status: CaseStatus,
    pub message: Option<String>,
    pub severity: Option<MismatchSeverity>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MismatchSeverity {
    Minor,
    Noticeable,
    Major,
}

#[derive(Clone, Debug)]
pub struct CaseState {
    total_attempts: u32,
    started: bool,
    start_time: Option<Instant>,
    attempts_started: u32,
    remaining_attempts: u32,
    final_result: Option<CaseResult>,
}

impl CaseState {
    pub fn new(total_attempts: u32) -> Self {
        Self {
            total_attempts,
            started: false,
            start_time: None,
            attempts_started: 0,
            remaining_attempts: total_attempts,
            final_result: None,
        }
    }

    pub fn begin_attempt(&mut self) -> u32 {
        if !self.started {
            self.started = true;
            self.start_time = Some(Instant::now());
        }
        self.attempts_started += 1;
        if self.remaining_attempts > 0 {
            self.remaining_attempts -= 1;
        }
        self.attempts_started
    }

    pub fn total_attempts(&self) -> u32 {
        self.total_attempts
    }

    pub fn attempts_left(&self) -> u32 {
        self.remaining_attempts
    }

    pub fn is_finished(&self) -> bool {
        self.final_result.is_some()
    }

    pub fn finalize(&mut self, result: CaseResult) {
        self.final_result = Some(result);
        self.remaining_attempts = 0;
    }

    pub fn duration_ms(&self) -> Option<f64> {
        self.start_time
            .map(|start| start.elapsed().as_secs_f64() * 1000.0)
    }
}

#[derive(Clone, Debug)]
pub struct CompareMeta {
    pub case_index: usize,
    pub case_key: String,
    pub browser: BrowserKind,
    pub actual_path: camino::Utf8PathBuf,
    pub diff_path: camino::Utf8PathBuf,
}

impl CompareMeta {
    pub fn new(
        case_index: usize,
        case_key: String,
        browser: BrowserKind,
        actual_path: camino::Utf8PathBuf,
        diff_path: camino::Utf8PathBuf,
    ) -> Self {
        Self {
            case_index,
            case_key,
            browser,
            actual_path,
            diff_path,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RenderOutcome {
    Screenshot(Screenshot),
    Error(CaseResult),
}
