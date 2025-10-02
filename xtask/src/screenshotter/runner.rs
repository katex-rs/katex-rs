use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::{Context, Report, Result, bail, eyre};
use indicatif::ProgressBar;
use serde_json::Value as JsonValue;
use thirtyfour::WebDriver;
use tokio::task::{JoinSet, spawn_blocking};
use tokio::time::sleep;

use crate::screenshotter::args::{
    BASELINE_DIR, BrowserKind, DEFAULT_BROWSERS, DIFF_DIR, HTML_DIR, NEW_DIR, PAGE_PATH,
    ScreenshotterArgs,
};
use crate::screenshotter::build::{ensure_katex_dist_assets, ensure_wasm_artifacts};
use crate::screenshotter::compare::{
    CompareJob, CompareOutcome, CompareSettings, CompareWorkResult, compare_images,
    preload_baselines, run_compare_job,
};
use crate::screenshotter::dataset::{filter_cases, load_cases, workspace_root};
use crate::screenshotter::fs_utils::sync_artifact;
use crate::screenshotter::logger::{Logger, WarnLevel, summarize_failures};
use crate::screenshotter::models::{
    CaseResult, CaseState, CaseStatus, CompareMeta, HtmlSnapshot, MismatchSeverity, RenderOutcome,
    Screenshot, TestCase,
};
use crate::screenshotter::server::start_static_server;
use crate::screenshotter::viewport::{
    calibrate_browser_viewport, configure_chrome_viewport, normalize_viewport_screenshot,
};
use crate::screenshotter::webdriver::{ensure_output_dirs, start_webdriver};

struct BrowserRunConfig<'a> {
    args: &'a ScreenshotterArgs,
    wait_ms: u64,
    browser: BrowserKind,
    server_url: &'a str,
    compare_settings: CompareSettings,
}

struct PendingFallback {
    case_index: usize,
    case_key: String,
    browser: BrowserKind,
    screenshot: Screenshot,
    outcome: CompareOutcome,
}

pub fn run(mut args: ScreenshotterArgs) -> Result<()> {
    let logger = Logger::new();

    if args.attempts == 0 {
        bail!("attempts must be greater than zero");
    }

    if args.browsers.is_empty() {
        args.browsers.extend(DEFAULT_BROWSERS);
    }

    let mut seen = HashSet::new();
    args.browsers.retain(|b| seen.insert(*b));

    if !cfg!(target_os = "macos")
        && args
            .browsers
            .iter()
            .any(|browser| matches!(browser, BrowserKind::Safari))
    {
        logger.warn("Safari automation is only supported on macOS hosts; skipping Safari.");
        args.browsers
            .retain(|browser| !matches!(browser, BrowserKind::Safari));
    }

    if args.browsers.is_empty() {
        bail!("no supported browsers remain after applying host-specific filters");
    }

    if args.webdriver.is_some() && args.browsers.len() > 1 {
        bail!("--webdriver can only be used when targeting a single browser");
    }

    let root = workspace_root()?;
    ensure_output_dirs(&root)?;
    ensure_wasm_artifacts(&root, args.build)?;
    ensure_katex_dist_assets(&root, args.build)?;

    let cases = load_cases(&root, &args)?;
    let cases = filter_cases(cases, &args);
    if cases.is_empty() {
        bail!("no screenshotter cases matched the provided filters");
    }

    let compare_settings = args.tolerance.settings();

    logger.info(format!("Loaded {} cases.", cases.len()));
    logger.info(compare_settings.summary());

    let wait_ms = if args.wait <= 0.0 {
        0
    } else {
        (args.wait * 1000.0).round() as u64
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let browsers = args.browsers.clone();
    let logger_clone = logger.clone();
    let cases_clone = cases.clone();
    let root_clone = root.clone();
    let compare_settings_clone = compare_settings;

    runtime.block_on(async move {
        let (addr, shutdown_tx, server_handle) =
            start_static_server(&logger_clone, &root_clone, args.port).await?;
        let server_url = format!("http://{}:{}", addr.ip(), addr.port());
        logger_clone.info(format!("Static assets available at {server_url}"));

        let mut result = Ok(());
        for browser in browsers {
            logger_clone.blank();
            logger_clone.browser_banner(browser, cases_clone.len());

            if let Err(err) = run_browser(
                logger_clone.clone(),
                root_clone.clone(),
                &cases_clone,
                BrowserRunConfig {
                    args: &args,
                    wait_ms,
                    browser,
                    server_url: &server_url,
                    compare_settings: compare_settings_clone,
                },
            )
            .await
            {
                result = Err(err);
                break;
            }
        }

        let _ = shutdown_tx.send(());
        if let Err(err) = server_handle.await {
            logger_clone.warn(format!("Static server task panicked: {err}"));
        }

        result
    })
}

async fn run_browser(
    logger: Logger,
    root: Utf8PathBuf,
    cases: &[TestCase],
    config: BrowserRunConfig<'_>,
) -> Result<()> {
    let BrowserRunConfig {
        args,
        wait_ms,
        browser,
        server_url,
        compare_settings,
    } = config;
    let (driver, child, webdriver_url) = start_webdriver(args, browser).await?;
    logger.info(format!(
        "Connected to {} WebDriver at {webdriver_url}",
        browser
    ));

    calibrate_browser_viewport(&logger, &driver, browser)
        .await
        .context("failed to calibrate viewport")?;

    if matches!(browser, BrowserKind::Chrome) {
        configure_chrome_viewport(&driver)
            .await
            .context("failed to configure Chrome viewport")?;
    }

    let base_url = format!("{server_url}{PAGE_PATH}");
    driver.goto(&base_url).await.map_err(Report::from)?;

    wait_for_run_case(&driver, Duration::from_millis(args.timeout)).await?;

    let baseline_dir = root.join(BASELINE_DIR);
    let new_dir = root.join(NEW_DIR);
    let diff_dir = root.join(DIFF_DIR);
    let baseline_cache = preload_baselines(&baseline_dir, cases, browser).await?;
    let timeout = Duration::from_millis(args.timeout);

    let mut failures: Vec<(String, CaseResult)> = Vec::new();
    let mut timings = Vec::new();
    let mut case_states: Vec<CaseState> = (0..cases.len())
        .map(|_| CaseState::new(args.attempts))
        .collect();
    let mut queue: VecDeque<usize> = (0..cases.len()).collect();
    let mut compare_tasks: JoinSet<(CompareMeta, Result<CompareWorkResult>)> = JoinSet::new();
    let mut fallback_tasks: VecDeque<PendingFallback> = VecDeque::new();
    let concurrency_limit = std::thread::available_parallelism()
        .map(|n| n.get().max(1))
        .unwrap_or(4);

    let started_at = Instant::now();
    let progress = logger.progress_group(cases.len(), browser);
    let capture_progress = progress.as_ref().map(|group| group.capture().clone());
    let compare_progress = progress.as_ref().map(|group| group.compare().clone());

    while !queue.is_empty() || !compare_tasks.is_empty() || !fallback_tasks.is_empty() {
        if let Some(pending) = fallback_tasks.pop_front() {
            handle_js_fallback(
                &logger,
                compare_progress.as_ref(),
                &driver,
                root.as_ref(),
                &cases[pending.case_index],
                wait_ms,
                timeout,
                pending,
                &mut case_states,
                &mut failures,
                &mut timings,
                args.html_on_failure,
                compare_settings,
            )
            .await?;
            continue;
        }

        if let Some(case_index) = queue.pop_front() {
            if case_states[case_index].is_finished() {
                continue;
            }

            if compare_tasks.len() >= concurrency_limit {
                queue.push_front(case_index);
                if let Some((failed_index, _)) = process_next_compare(
                    &logger,
                    compare_progress.as_ref(),
                    &mut compare_tasks,
                    &mut case_states,
                    &mut queue,
                    &mut failures,
                    &mut timings,
                    &mut fallback_tasks,
                    args.allow_js_fallback,
                )
                .await?
                {
                    maybe_dump_case_html(
                        &logger,
                        compare_progress.as_ref(),
                        &driver,
                        root.as_ref(),
                        &cases[failed_index],
                        browser,
                        wait_ms,
                        timeout,
                        args.html_on_failure,
                    )
                    .await;
                }
                continue;
            }

            if case_states[case_index].attempts_left() == 0 {
                continue;
            }

            let state = &mut case_states[case_index];
            let attempt = state.begin_attempt();
            if attempt == 1 {
                if let Some(pb) = &capture_progress {
                    pb.inc(1);
                }
                logger.case_intro(
                    capture_progress.as_ref(),
                    case_index,
                    cases.len(),
                    &cases[case_index].key,
                    browser,
                );
            } else {
                logger.detail(
                    capture_progress.as_ref(),
                    format!("attempt {attempt}/{total}", total = state.total_attempts()),
                );
            }

            match render_case(
                &logger,
                capture_progress.as_ref(),
                &driver,
                &cases[case_index],
                timeout,
                wait_ms,
                browser,
            )
            .await
            {
                Ok(RenderOutcome::Screenshot(screenshot)) => {
                    let baseline_path = baseline_dir.join(format!(
                        "{}{}",
                        cases[case_index].key,
                        browser.screenshot_suffix()
                    ));
                    let actual_path = new_dir.join(format!(
                        "{}{}",
                        cases[case_index].key,
                        browser.screenshot_suffix()
                    ));
                    let diff_path = diff_dir.join(format!(
                        "{}{}",
                        cases[case_index].key,
                        browser.diff_suffix()
                    ));

                    let job = CompareJob {
                        screenshot,
                        baseline: baseline_cache.get(&cases[case_index].key).cloned(),
                        baseline_path,
                        settings: compare_settings,
                    };
                    let meta = CompareMeta::new(
                        case_index,
                        cases[case_index].key.clone(),
                        browser,
                        actual_path,
                        diff_path,
                    );

                    compare_tasks.spawn(async move {
                        let compare_res = spawn_blocking(move || run_compare_job(job)).await;
                        let compare_res = match compare_res {
                            Ok(result) => result,
                            Err(err) => Err(eyre!(err)),
                        };
                        (meta, compare_res)
                    });
                }
                Ok(RenderOutcome::Error(case_result)) => {
                    if case_states[case_index].attempts_left() > 0 {
                        if let Some(message) = &case_result.message {
                            logger.retrying(
                                capture_progress.as_ref(),
                                format!("retrying: {message}"),
                            );
                        }
                        queue.push_back(case_index);
                        sleep(Duration::from_millis(200)).await;
                    } else {
                        let message = case_result
                            .message
                            .clone()
                            .unwrap_or_else(|| "unknown failure".to_owned());
                        logger.case_failure(
                            compare_progress.as_ref(),
                            case_result.status,
                            &cases[case_index].key,
                            browser,
                            message.clone(),
                        );
                        failures.push((
                            format!("{} [{}]", cases[case_index].key, browser),
                            case_result.clone(),
                        ));
                        case_states[case_index].finalize(case_result);
                        maybe_dump_case_html(
                            &logger,
                            compare_progress.as_ref(),
                            &driver,
                            root.as_ref(),
                            &cases[case_index],
                            browser,
                            wait_ms,
                            timeout,
                            args.html_on_failure,
                        )
                        .await;
                    }
                }
                Err(err) => {
                    let message = err.to_string();

                    if case_states[case_index].attempts_left() > 0 {
                        logger.retrying(capture_progress.as_ref(), format!("retrying: {message}"));
                        queue.push_back(case_index);
                        sleep(Duration::from_millis(200)).await;
                    } else {
                        logger.case_failure(
                            compare_progress.as_ref(),
                            CaseStatus::Error,
                            &cases[case_index].key,
                            browser,
                            message.clone(),
                        );
                        let failure = CaseResult {
                            status: CaseStatus::Error,
                            message: Some(message.clone()),
                            severity: None,
                        };
                        failures.push((
                            format!("{} [{}]", cases[case_index].key, browser),
                            failure.clone(),
                        ));
                        case_states[case_index].finalize(failure);
                        maybe_dump_case_html(
                            &logger,
                            compare_progress.as_ref(),
                            &driver,
                            root.as_ref(),
                            &cases[case_index],
                            browser,
                            wait_ms,
                            timeout,
                            args.html_on_failure,
                        )
                        .await;
                    }
                }
            }
        } else if !compare_tasks.is_empty() {
            if let Some((failed_index, _)) = process_next_compare(
                &logger,
                compare_progress.as_ref(),
                &mut compare_tasks,
                &mut case_states,
                &mut queue,
                &mut failures,
                &mut timings,
                &mut fallback_tasks,
                args.allow_js_fallback,
            )
            .await?
            {
                maybe_dump_case_html(
                    &logger,
                    compare_progress.as_ref(),
                    &driver,
                    root.as_ref(),
                    &cases[failed_index],
                    browser,
                    wait_ms,
                    timeout,
                    args.html_on_failure,
                )
                .await;
            }
        }
    }

    while !compare_tasks.is_empty() {
        if let Some((failed_index, _)) = process_next_compare(
            &logger,
            compare_progress.as_ref(),
            &mut compare_tasks,
            &mut case_states,
            &mut queue,
            &mut failures,
            &mut timings,
            &mut fallback_tasks,
            args.allow_js_fallback,
        )
        .await?
        {
            maybe_dump_case_html(
                &logger,
                compare_progress.as_ref(),
                &driver,
                root.as_ref(),
                &cases[failed_index],
                browser,
                wait_ms,
                timeout,
                args.html_on_failure,
            )
            .await;
        }
    }

    while let Some(pending) = fallback_tasks.pop_front() {
        handle_js_fallback(
            &logger,
            compare_progress.as_ref(),
            &driver,
            root.as_ref(),
            &cases[pending.case_index],
            wait_ms,
            timeout,
            pending,
            &mut case_states,
            &mut failures,
            &mut timings,
            args.html_on_failure,
            compare_settings,
        )
        .await?;
    }

    if let Some(mut child) = child {
        let _ = child.kill();
        let _ = child.wait();
    }

    let elapsed = started_at.elapsed().as_secs_f64();
    let summary_line = format!(
        "{} cases in {:.2}s (avg {:.2}ms)",
        cases.len(),
        elapsed,
        timings.iter().copied().sum::<f64>() / timings.len().max(1) as f64
    );

    if let Some(group) = &progress {
        group.finish_capture();
    }

    if failures.is_empty() {
        logger.finish_progress(compare_progress.clone(), summary_line.clone());
        logger.info(summary_line);
        logger.success(format!("All cases passed for {}", browser));
        Ok(())
    } else {
        logger.finish_progress(
            compare_progress.clone(),
            format!("{} issues – {summary_line}", failures.len()),
        );
        logger.info(summary_line.clone());
        let severity = summarize_failures(&logger, &failures);
        if let Some(level) = severity {
            match level {
                WarnLevel::Low => logger.warn_with_progress(
                    None,
                    WarnLevel::Low,
                    format!(
                        "{}/{} cases had minor differences for {} (new={}, diff={})",
                        failures.len(),
                        cases.len(),
                        browser,
                        root.join(NEW_DIR),
                        root.join(DIFF_DIR)
                    ),
                ),
                WarnLevel::Medium => logger.warn_with_progress(
                    None,
                    WarnLevel::Medium,
                    format!(
                        "{}/{} cases failed for {} (new={}, diff={})",
                        failures.len(),
                        cases.len(),
                        browser,
                        root.join(NEW_DIR),
                        root.join(DIFF_DIR)
                    ),
                ),
                WarnLevel::High => logger.error(format!(
                    "{}/{} cases failed for {} (new={}, diff={})",
                    failures.len(),
                    cases.len(),
                    browser,
                    root.join(NEW_DIR),
                    root.join(DIFF_DIR)
                )),
            };
        }

        bail!("screenshotter detected mismatches");
    }
}

async fn process_next_compare(
    logger: &Logger,
    compare_progress: Option<&ProgressBar>,
    compare_tasks: &mut JoinSet<(CompareMeta, Result<CompareWorkResult>)>,
    case_states: &mut [CaseState],
    queue: &mut VecDeque<usize>,
    failures: &mut Vec<(String, CaseResult)>,
    timings: &mut Vec<f64>,
    fallback_tasks: &mut VecDeque<PendingFallback>,
    allow_js_fallback: bool,
) -> Result<Option<(usize, CaseResult)>> {
    if let Some(join_result) = compare_tasks.join_next().await {
        let (meta, outcome_result) = join_result.map_err(|err| eyre!(err))?;
        let CompareMeta {
            case_index,
            case_key,
            browser,
            actual_path,
            diff_path,
        } = meta;

        let state = &mut case_states[case_index];
        if state.is_finished() {
            return Ok(None);
        }

        match outcome_result {
            Ok(work) => {
                let CompareWorkResult {
                    screenshot,
                    outcome,
                } = work;

                sync_artifact(diff_path.as_ref(), outcome.diff_image.as_deref()).await?;

                let should_write_actual = !outcome.equal || outcome.note.is_some();
                let actual_bytes = should_write_actual.then_some(screenshot.png.as_slice());
                sync_artifact(actual_path.as_ref(), actual_bytes).await?;

                if outcome.equal {
                    logger.case_pass(compare_progress, &case_key, browser, state.duration_ms());
                    state.finalize(CaseResult {
                        status: CaseStatus::Pass,
                        message: None,
                        severity: None,
                    });
                    if let Some(duration) = state.duration_ms() {
                        timings.push(duration);
                    }
                    return Ok(None);
                }

                let severity = outcome.severity.unwrap_or(MismatchSeverity::Major);
                let message = outcome
                    .note
                    .clone()
                    .or_else(|| {
                        outcome
                            .diff_pixels
                            .map(|p| format!("Differs from baseline (diff pixels: {p})"))
                    })
                    .unwrap_or_else(|| "Screenshot differs from baseline".to_owned());

                if state.attempts_left() > 0 {
                    logger.retrying(compare_progress, format!("retrying: {message}"));
                    queue.push_back(case_index);
                    sleep(Duration::from_millis(50)).await;
                    return Ok(None);
                }

                if allow_js_fallback {
                    fallback_tasks.push_back(PendingFallback {
                        case_index,
                        case_key: case_key.clone(),
                        browser,
                        screenshot,
                        outcome,
                    });
                    return Ok(None);
                }

                logger.case_mismatch(
                    compare_progress,
                    &case_key,
                    browser,
                    severity,
                    message.clone(),
                );
                let failure = CaseResult {
                    status: CaseStatus::Mismatch,
                    message: Some(message.clone()),
                    severity: Some(severity),
                };
                failures.push((format!("{case_key} [{browser}]"), failure.clone()));
                state.finalize(failure.clone());
                return Ok(Some((case_index, failure)));
            }
            Err(err) => {
                let message = err.to_string();
                if state.attempts_left() > 0 {
                    logger.retrying(compare_progress, format!("retrying: {message}"));
                    queue.push_back(case_index);
                    sleep(Duration::from_millis(200)).await;
                    return Ok(None);
                }

                logger.case_failure(
                    compare_progress,
                    CaseStatus::Error,
                    &case_key,
                    browser,
                    message.clone(),
                );
                let failure = CaseResult {
                    status: CaseStatus::Error,
                    message: Some(message.clone()),
                    severity: None,
                };
                failures.push((format!("{case_key} [{browser}]"), failure.clone()));
                state.finalize(failure.clone());
                return Ok(Some((case_index, failure)));
            }
        }
    }

    Ok(None)
}

async fn handle_js_fallback(
    logger: &Logger,
    compare_progress: Option<&ProgressBar>,
    driver: &WebDriver,
    root: &Utf8Path,
    case: &TestCase,
    wait_ms: u64,
    timeout: Duration,
    pending: PendingFallback,
    case_states: &mut [CaseState],
    failures: &mut Vec<(String, CaseResult)>,
    timings: &mut Vec<f64>,
    capture_html: bool,
    compare_settings: CompareSettings,
) -> Result<()> {
    let PendingFallback {
        case_index,
        case_key,
        browser,
        screenshot,
        outcome,
    } = pending;

    let reason = outcome
        .note
        .clone()
        .or_else(|| {
            outcome
                .baseline_missing
                .then_some("Baseline missing".to_owned())
        })
        .unwrap_or_else(|| "Screenshot differs from baseline".to_owned());

    logger.warn_with_progress(
        compare_progress,
        WarnLevel::Low,
        format!("{case_key} ({browser}) {reason}; comparing against JS implementation"),
    );

    match render_case_with_impl(
        logger,
        compare_progress,
        driver,
        case,
        timeout,
        wait_ms,
        browser,
        Some("js"),
    )
    .await
    {
        Ok(RenderOutcome::Screenshot(js_screenshot)) => {
            let comparison =
                compare_images(&screenshot.image, &js_screenshot.image, compare_settings)?;
            if comparison.equal {
                let state = &mut case_states[case_index];
                logger.case_pass(compare_progress, &case_key, browser, state.duration_ms());
                logger.warn_with_progress(
                    compare_progress,
                    WarnLevel::Low,
                    format!("{case_key} ({browser}) matched JS output; treating as pass"),
                );
                state.finalize(CaseResult {
                    status: CaseStatus::Pass,
                    message: None,
                    severity: None,
                });
                if let Some(duration) = state.duration_ms() {
                    timings.push(duration);
                }
                return Ok(());
            }

            let severity = comparison.severity.unwrap_or(MismatchSeverity::Major);
            let fallback_note = comparison
                .note
                .clone()
                .unwrap_or_else(|| "Screenshot differs from JS implementation".to_owned());
            let message = format!("{fallback_note} (vs JS fallback)");
            logger.case_mismatch(
                compare_progress,
                &case_key,
                browser,
                severity,
                message.clone(),
            );
            let failure = CaseResult {
                status: CaseStatus::Mismatch,
                message: Some(message.clone()),
                severity: Some(severity),
            };
            failures.push((format!("{case_key} [{browser}]"), failure.clone()));
            case_states[case_index].finalize(failure);
            maybe_dump_case_html(
                logger,
                compare_progress,
                driver,
                root,
                case,
                browser,
                wait_ms,
                timeout,
                capture_html,
            )
            .await;
        }
        Ok(RenderOutcome::Error(case_result)) => {
            let message = case_result
                .message
                .clone()
                .unwrap_or_else(|| "JS fallback render error".to_owned());
            let failure = CaseResult {
                status: CaseStatus::Error,
                message: Some(format!("JS fallback error: {message}")),
                severity: None,
            };
            logger.case_failure(
                compare_progress,
                failure.status,
                &case_key,
                browser,
                failure.message.clone().unwrap(),
            );
            failures.push((format!("{case_key} [{browser}]"), failure.clone()));
            case_states[case_index].finalize(failure);
            maybe_dump_case_html(
                logger,
                compare_progress,
                driver,
                root,
                case,
                browser,
                wait_ms,
                timeout,
                capture_html,
            )
            .await;
        }
        Err(err) => {
            let message = err.to_string();
            let failure = CaseResult {
                status: CaseStatus::Error,
                message: Some(format!("JS fallback failure: {message}")),
                severity: None,
            };
            logger.case_failure(
                compare_progress,
                failure.status,
                &case_key,
                browser,
                failure.message.clone().unwrap(),
            );
            failures.push((format!("{case_key} [{browser}]"), failure.clone()));
            case_states[case_index].finalize(failure);
            maybe_dump_case_html(
                logger,
                compare_progress,
                driver,
                root,
                case,
                browser,
                wait_ms,
                timeout,
                capture_html,
            )
            .await;
        }
    }

    Ok(())
}

async fn invoke_run_case(
    driver: &WebDriver,
    case: &TestCase,
    timeout: Duration,
    wait_ms: u64,
    impl_override: Option<&str>,
) -> Result<Result<(), CaseResult>> {
    let mut args = Vec::new();
    args.push(case.payload.clone());
    if let Some(mode) = impl_override {
        args.push(JsonValue::String(mode.to_string().into()));
    }

    let run_result = driver
        .execute_async(RUN_CASE_SCRIPT, args)
        .await
        .map_err(Report::from)?
        .convert::<JsonValue>()?;

    if let Some(state) = run_result.get("state").and_then(JsonValue::as_str)
        && state.eq_ignore_ascii_case("error")
    {
        let message = run_result
            .get("message")
            .and_then(JsonValue::as_str)
            .unwrap_or("render error")
            .to_owned();

        return Ok(Err(CaseResult {
            status: CaseStatus::Error,
            message: Some(message),
            severity: None,
        }));
    }

    wait_for_ready_state(driver, timeout).await?;

    if wait_ms > 0 {
        sleep(Duration::from_millis(wait_ms)).await;
    }

    Ok(Ok(()))
}

async fn render_case(
    logger: &Logger,
    progress: Option<&ProgressBar>,
    driver: &WebDriver,
    case: &TestCase,
    timeout: Duration,
    wait_ms: u64,
    browser: BrowserKind,
) -> Result<RenderOutcome> {
    render_case_with_impl(
        logger, progress, driver, case, timeout, wait_ms, browser, None,
    )
    .await
}

async fn render_case_with_impl(
    logger: &Logger,
    progress: Option<&ProgressBar>,
    driver: &WebDriver,
    case: &TestCase,
    timeout: Duration,
    wait_ms: u64,
    browser: BrowserKind,
    impl_override: Option<&str>,
) -> Result<RenderOutcome> {
    match invoke_run_case(driver, case, timeout, wait_ms, impl_override).await? {
        Ok(()) => {
            let screenshot = capture_case_screenshot(logger, progress, driver, browser).await?;
            Ok(RenderOutcome::Screenshot(screenshot))
        }
        Err(case_result) => Ok(RenderOutcome::Error(case_result)),
    }
}

async fn capture_case_screenshot(
    logger: &Logger,
    progress: Option<&ProgressBar>,
    driver: &WebDriver,
    browser: BrowserKind,
) -> Result<Screenshot> {
    let raw_screenshot = driver.screenshot_as_png().await.map_err(Report::from)?;
    normalize_viewport_screenshot(logger, progress, &raw_screenshot, browser)
}

async fn wait_for_ready_state(driver: &WebDriver, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        let ready: bool = driver
            .execute("return window.__ready === true;", Vec::<JsonValue>::new())
            .await
            .map_err(Report::from)?
            .convert()?;
        if ready {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            bail!(
                "timed out after {}ms waiting for window.__ready",
                timeout.as_millis()
            );
        }
        sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_for_run_case(driver: &WebDriver, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        let result: bool = driver
            .execute(
                "return typeof window.runCase === 'function';",
                Vec::<JsonValue>::new(),
            )
            .await
            .map_err(Report::from)?
            .convert()?;
        if result {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            let status: Option<JsonValue> = driver
                .execute(
                    "return typeof window.__status === 'object' ? window.__status : null;",
                    Vec::<JsonValue>::new(),
                )
                .await
                .map_err(Report::from)?
                .convert()?;
            if let Some(status) = status {
                bail!(
                    "runCase helper did not become available within {}ms (status: {})",
                    timeout.as_millis(),
                    status
                );
            } else {
                bail!(
                    "runCase helper did not become available within {}ms",
                    timeout.as_millis()
                );
            }
        }
        sleep(Duration::from_millis(50)).await;
    }
}

struct HtmlDumpResult {
    saved_paths: Vec<Utf8PathBuf>,
    warnings: Vec<String>,
}

async fn maybe_dump_case_html(
    logger: &Logger,
    progress: Option<&ProgressBar>,
    driver: &WebDriver,
    root: &Utf8Path,
    case: &TestCase,
    browser: BrowserKind,
    wait_ms: u64,
    timeout: Duration,
    enabled: bool,
) {
    if !enabled {
        return;
    }

    match dump_case_html(driver, root, case, browser, wait_ms, timeout).await {
        Ok(result) => {
            if !result.saved_paths.is_empty() {
                let joined = result
                    .saved_paths
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                logger.detail(
                    progress,
                    format!(
                        "Captured HTML artifacts for {} [{}]: {joined}",
                        case.key, browser
                    ),
                );
            }
            for warning in result.warnings {
                logger.warn_with_progress(
                    progress,
                    WarnLevel::Low,
                    format!("{} [{}]: {warning}", case.key, browser),
                );
            }
        }
        Err(err) => {
            logger.warn_with_progress(
                progress,
                WarnLevel::Low,
                format!(
                    "{} [{}]: failed to capture HTML artifacts: {err}",
                    case.key, browser
                ),
            );
        }
    }
}

async fn dump_case_html(
    driver: &WebDriver,
    root: &Utf8Path,
    case: &TestCase,
    browser: BrowserKind,
    wait_ms: u64,
    timeout: Duration,
) -> Result<HtmlDumpResult> {
    let mut result = HtmlDumpResult {
        saved_paths: Vec::new(),
        warnings: Vec::new(),
    };

    if let Some(snapshot) = capture_html_snapshot(driver).await? {
        let path = write_html_artifact(root, case, browser, &snapshot).await?;
        result.saved_paths.push(path);
    } else {
        result
            .warnings
            .push("captureHtmlSnapshot helper is unavailable".to_owned());
    }

    let alt_impl = "js";
    match invoke_run_case(driver, case, timeout, wait_ms, Some(alt_impl)).await? {
        Ok(()) => {
            if let Some(snapshot) = capture_html_snapshot(driver).await? {
                let path = write_html_artifact(root, case, browser, &snapshot).await?;
                if !result.saved_paths.iter().any(|p| p == &path) {
                    result.saved_paths.push(path);
                }
            } else {
                result.warnings.push(format!(
                    "captureHtmlSnapshot helper returned null after rendering with {alt_impl}",
                ));
            }
        }
        Err(case_result) => {
            let message = case_result
                .message
                .clone()
                .unwrap_or_else(|| "render error".to_owned());
            result.warnings.push(format!(
                "{alt_impl} implementation reported an error: {message}",
            ));
            if let Some(snapshot) = capture_html_snapshot(driver).await? {
                let path = write_html_artifact(root, case, browser, &snapshot).await?;
                if !result.saved_paths.iter().any(|p| p == &path) {
                    result.saved_paths.push(path);
                }
            }
        }
    }

    Ok(result)
}

async fn write_html_artifact(
    root: &Utf8Path,
    case: &TestCase,
    browser: BrowserKind,
    snapshot: &HtmlSnapshot,
) -> Result<Utf8PathBuf> {
    let impl_label = snapshot
        .implementation
        .as_deref()
        .unwrap_or("default")
        .to_lowercase();
    let sanitized_key = sanitized_case_key(&case.key);
    let file_name = format!("{}-{}-{}.html", sanitized_key, browser.slug(), impl_label);
    let path = root.join(HTML_DIR).join(file_name);
    let document = build_html_document(&case.key, snapshot, &impl_label);
    let bytes = document.into_bytes();
    sync_artifact(path.as_ref(), Some(bytes.as_slice())).await?;
    Ok(path)
}

fn sanitized_case_key(key: &str) -> String {
    key.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect()
}

fn build_html_document(case_key: &str, snapshot: &HtmlSnapshot, impl_label: &str) -> String {
    let status = snapshot.status.as_deref().unwrap_or("unknown");
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\" />\n  <title>{case_key} [{impl_label}]</title>\n  <style>body {{ font-family: sans-serif; margin: 1rem; }}\n  header {{ margin-bottom: 1rem; }}\n  section {{ margin-bottom: 1rem; }}\n  .math {{ padding: 1rem; border: 1px solid #ccc; }}\n  </style>\n</head>\n<body>\n  <header>\n    <h1>{case_key}</h1>\n    <p><strong>Implementation:</strong> {impl_label}</p>\n    <p><strong>Status:</strong> {status}</p>\n  </header>\n  <section>\n    <h2>Pre</h2>\n    <div id=\"pre\">{}</div>\n  </section>\n  <section class=\"math\">\n    <h2>Math</h2>\n    <div id=\"math\">{}</div>\n  </section>\n  <section>\n    <h2>Post</h2>\n    <div id=\"post\">{}</div>\n  </section>\n</body>\n</html>\n",
        snapshot.pre_html, snapshot.math_html, snapshot.post_html
    )
}

async fn capture_html_snapshot(driver: &WebDriver) -> Result<Option<HtmlSnapshot>> {
    let snapshot: Option<JsonValue> = driver
        .execute(CAPTURE_HTML_SCRIPT, Vec::<JsonValue>::new())
        .await
        .map_err(Report::from)?
        .convert()?;
    if let Some(value) = snapshot {
        Ok(Some(HtmlSnapshot::from_json(value)?))
    } else {
        Ok(None)
    }
}

const RUN_CASE_SCRIPT: &str = r#"
    const payload = arguments[0];
    const implMode = arguments.length > 2 ? arguments[1] : null;
    const done = arguments[arguments.length - 1];
    const hasRenderWithImpl = typeof window.renderWithImpl === 'function';
    const run = () => {
        const implValue = typeof implMode === 'string' ? implMode : null;
        if (implValue && hasRenderWithImpl) {
            return window.renderWithImpl(implValue, payload);
        }
        if (typeof window.runCase === 'function') {
            return window.runCase(payload);
        }
        throw new Error('window.runCase is not available');
    };
    Promise.resolve()
        .then(run)
        .then(result => done(result || {}))
        .catch(err => {
            const message = err && err.message ? err.message : String(err);
            const stack = err && err.stack ? err.stack : null;
            done({ state: 'error', message, stack });
        });
"#;

const CAPTURE_HTML_SCRIPT: &str = r#"
    if (typeof window.captureHtmlSnapshot !== 'function') {
        return null;
    }
    return window.captureHtmlSnapshot();
"#;
