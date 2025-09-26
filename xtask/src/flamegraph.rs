use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufReader, Cursor};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{ArgAction, Args, Subcommand, ValueEnum};
use inferno::collapse::Collapse;
use inferno::collapse::perf::Folder as PerfFolder;
use inferno::flamegraph::{Options, from_reader};
use xshell::{Shell, cmd};

const PERF_CASES: &[&str] = &[
    "AccentsText",
    "ArrayMode",
    "GroupMacros",
    "MathBb",
    "SqrtRoot",
    "StretchyAccent",
    "Units",
];

#[derive(Args)]
pub struct FlamegraphArgs {
    #[command(subcommand)]
    command: FlamegraphCommand,
}

#[derive(Subcommand)]
enum FlamegraphCommand {
    /// Profile the native Criterion benchmark harness.
    Native(NativeArgs),
    /// Profile the WebAssembly renderer executed under Node.js.
    Wasm(WasmArgs),
    /// Profile the upstream JavaScript renderer (KaTeX/test/perf-test.js).
    Js(JsArgs),
}

#[derive(Args, Clone)]
struct PerfCommonArgs {
    /// Sampling frequency passed to `perf record`.
    #[arg(long, default_value_t = 999)]
    frequency: u32,
    /// Call graph strategy to use when collecting samples.
    #[arg(long, value_enum, default_value_t = CallGraph::Dwarf)]
    call_graph: CallGraph,
    /// Output location for the generated SVG flamegraph.
    #[arg(long)]
    output: Option<Utf8PathBuf>,
    /// Persist the raw `perf.data` capture to this path.
    #[arg(long)]
    perf_data: Option<Utf8PathBuf>,
    /// Override the title rendered at the top of the flamegraph.
    #[arg(long)]
    title: Option<String>,
    /// Open the generated SVG in the system viewer when profiling completes.
    #[arg(long)]
    open: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum CallGraph {
    Dwarf,
    FramePointer,
    Lbr,
}

impl CallGraph {
    fn as_flag(self) -> &'static str {
        match self {
            CallGraph::Dwarf => "dwarf",
            CallGraph::FramePointer => "fp",
            CallGraph::Lbr => "lbr",
        }
    }
}

#[derive(Args)]
struct NativeArgs {
    #[command(flatten)]
    common: PerfCommonArgs,
    /// Benchmark target to execute (defaults to Criterion's `perf` harness).
    #[arg(long, default_value = "perf")]
    bench: String,
    /// Restrict the Criterion benchmark to a single named test case.
    #[arg(long, value_name = "NAME")]
    case: Option<String>,
    /// Use an alternate Cargo profile (defaults to `profiling`).
    #[arg(long, default_value = "profiling")]
    profile: String,
    /// Skip the warm-up build step (`cargo bench --no-run`).
    #[arg(long)]
    skip_build: bool,
    /// Extra arguments passed verbatim to Criterion after the `--` separator.
    #[arg(last = true)]
    extra: Vec<String>,
}

#[derive(Args)]
struct WasmArgs {
    #[command(flatten)]
    common: PerfCommonArgs,
    /// Skip building the wasm bindings via `wasm-pack`.
    #[arg(long)]
    skip_build: bool,
    /// Skip installing npm dependencies for `crates/katex/tests/perf`.
    #[arg(long)]
    skip_npm: bool,
    /// Restrict execution to a subset of the benchmarked test cases.
    #[arg(long = "case", value_name = "NAME", num_args = 1.., action = ArgAction::Append)]
    cases: Vec<String>,
    /// Additional flags forwarded to the Node.js executable.
    #[arg(long = "node-arg", value_name = "ARG", num_args = 1.., action = ArgAction::Append)]
    node_args: Vec<String>,
    /// Additional arguments forwarded to `wasm-perf.js`.
    #[arg(long = "script-arg", value_name = "ARG", num_args = 1.., action = ArgAction::Append)]
    script_args: Vec<String>,
}

#[derive(Args)]
struct JsArgs {
    #[command(flatten)]
    common: PerfCommonArgs,
    /// Skip installing npm dependencies for the KaTeX checkout.
    #[arg(long)]
    skip_npm: bool,
    /// Additional flags forwarded to the Node.js executable.
    #[arg(long = "node-arg", value_name = "ARG", num_args = 1.., action = ArgAction::Append)]
    node_args: Vec<String>,
    /// Additional arguments forwarded to the upstream perf harness.
    #[arg(long = "script-arg", value_name = "ARG", num_args = 1.., action = ArgAction::Append)]
    script_args: Vec<String>,
}

pub fn run(args: FlamegraphArgs) -> Result<()> {
    match args.command {
        FlamegraphCommand::Native(args) => run_native(args),
        FlamegraphCommand::Wasm(args) => run_wasm(args),
        FlamegraphCommand::Js(args) => run_js(args),
    }
}

fn run_native(args: NativeArgs) -> Result<()> {
    ensure_command_available("perf")?;

    let root = workspace_root()?;
    ensure_dataset_available(&root)?;

    if !args.skip_build {
        let mut cargo = Command::new("cargo");
        cargo.current_dir(&root);
        cargo.args([
            "bench",
            "--bench",
            &args.bench,
            "--profile",
            &args.profile,
            "--no-run",
        ]);
        run_command(cargo).context("failed to pre-build Criterion benchmarks")?;
    }

    let mut command = Vec::new();
    command.push(OsString::from("cargo"));
    command.push(OsString::from("bench"));
    command.push(OsString::from("--bench"));
    command.push(OsString::from(&args.bench));
    command.push(OsString::from("--profile"));
    command.push(OsString::from(&args.profile));

    if args.case.is_some() || !args.extra.is_empty() {
        command.push(OsString::from("--"));
    }

    if let Some(case) = &args.case {
        validate_case(case)?;
        command.push(OsString::from("--bench"));
        command.push(OsString::from(case));
    }

    if !args.extra.is_empty() {
        for value in &args.extra {
            command.push(OsString::from(value));
        }
    }

    let output_svg = default_output_path(&root, &args.common.output, "native.svg");
    let perf_data = default_output_path(&root, &args.common.perf_data, "native.perf.data");
    let title = args
        .common
        .title
        .clone()
        .unwrap_or_else(|| format!("katex-rs native :: {}", args.bench));

    execute_perf(
        PerfInvocation::builder(&root, command)
            .frequency(args.common.frequency)
            .call_graph(args.common.call_graph)
            .perf_data(perf_data)
            .output(output_svg.clone())
            .title(title)
            .build(),
    )?;

    if args.common.open {
        open_file(&output_svg)?;
    }

    Ok(())
}

fn run_wasm(args: WasmArgs) -> Result<()> {
    ensure_command_available("perf")?;
    ensure_command_available("node")?;

    let root = workspace_root()?;
    ensure_dataset_available(&root)?;

    let wasm_dir = root.join("crates/katex");
    let perf_dir = wasm_dir.join("tests/perf");

    if !args.skip_build {
        ensure_command_available("wasm-pack")?;
        let sh = Shell::new()?;
        sh.change_dir(&wasm_dir);
        cmd!(
            sh,
            "wasm-pack build --release --target nodejs --features wasm"
        )
        .run()?;
    }

    if !args.skip_npm {
        ensure_command_available("npm")?;
        let sh = Shell::new()?;
        sh.change_dir(&perf_dir);
        cmd!(sh, "npm install").run()?;
    }

    if !args.cases.is_empty() {
        for case in &args.cases {
            validate_case(case)?;
        }
    }

    let script = perf_dir.join("wasm-perf.js");
    if !script.exists() {
        bail!("missing wasm benchmark script at {}", script);
    }

    let mut command = Vec::<OsString>::new();
    command.push(OsString::from("node"));
    for arg in &args.node_args {
        command.push(OsString::from(arg));
    }
    command.push(OsString::from(script.as_str()));
    for case in &args.cases {
        command.push(OsString::from("--case"));
        command.push(OsString::from(case));
    }
    for value in &args.script_args {
        command.push(OsString::from(value));
    }

    let output_svg = default_output_path(&root, &args.common.output, "wasm.svg");
    let perf_data = default_output_path(&root, &args.common.perf_data, "wasm.perf.data");
    let title = args
        .common
        .title
        .clone()
        .unwrap_or_else(|| "katex-rs wasm (Node.js)".to_owned());

    execute_perf(
        PerfInvocation::builder(&root, command)
            .frequency(args.common.frequency)
            .call_graph(args.common.call_graph)
            .perf_data(perf_data)
            .output(output_svg.clone())
            .title(title)
            .build(),
    )?;

    if args.common.open {
        open_file(&output_svg)?;
    }

    Ok(())
}

fn run_js(args: JsArgs) -> Result<()> {
    ensure_command_available("perf")?;
    ensure_command_available("node")?;

    let root = workspace_root()?;
    let katex_checkout = root.join("KaTeX");
    let perf_script = katex_checkout.join("test/perf-test.js");
    if !perf_script.exists() {
        bail!(
            "KaTeX perf harness not found at {}. Did you fetch the submodule?",
            perf_script
        );
    }

    if !args.skip_npm {
        ensure_command_available("npm")?;
        let sh = Shell::new()?;
        sh.change_dir(&katex_checkout);
        cmd!(sh, "npm install").run()?;
    }

    let mut command = Vec::<OsString>::new();
    command.push(OsString::from("node"));
    for arg in &args.node_args {
        command.push(OsString::from(arg));
    }
    command.push(OsString::from(perf_script.as_str()));
    for value in &args.script_args {
        command.push(OsString::from(value));
    }

    let output_svg = default_output_path(&root, &args.common.output, "js.svg");
    let perf_data = default_output_path(&root, &args.common.perf_data, "js.perf.data");
    let title = args
        .common
        .title
        .clone()
        .unwrap_or_else(|| "KaTeX JavaScript reference".to_owned());

    execute_perf(
        PerfInvocation::builder(&katex_checkout, command)
            .frequency(args.common.frequency)
            .call_graph(args.common.call_graph)
            .perf_data(perf_data)
            .output(output_svg.clone())
            .title(title)
            .build(),
    )?;

    if args.common.open {
        open_file(&output_svg)?;
    }

    Ok(())
}

fn validate_case(case: &str) -> Result<()> {
    if PERF_CASES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(case))
    {
        Ok(())
    } else {
        bail!(
            "unknown test case `{case}`. Available cases: {}",
            PERF_CASES.join(", ")
        );
    }
}

fn default_output_path(
    root: &Utf8Path,
    override_path: &Option<Utf8PathBuf>,
    default_file: &str,
) -> Utf8PathBuf {
    if let Some(path) = override_path {
        path.clone()
    } else {
        let dir = root.join("target/flamegraphs");
        Utf8PathBuf::from_path_buf(dir.join(default_file).into_std_path_buf())
            .expect("valid UTF-8 path")
    }
}

fn workspace_root() -> Result<Utf8PathBuf> {
    let manifest_dir = Utf8PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    manifest_dir
        .parent()
        .map(|path| path.to_owned())
        .ok_or_else(|| anyhow!("failed to determine workspace root"))
}

fn ensure_dataset_available(root: &Utf8Path) -> Result<()> {
    let data_path = root.join("KaTeX/test/screenshotter/ss_data.yaml");
    if !data_path.exists() {
        bail!(
            "KaTeX screenshotter dataset not found at {}. Run `git submodule update --init --recursive` first.",
            data_path
        );
    }
    Ok(())
}

fn ensure_command_available(program: &str) -> Result<()> {
    let status = Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("command `{program}` exited with status {status}"),
        Err(err) => bail!("failed to execute `{program}`: {err}"),
    }
}

struct PerfInvocation {
    working_dir: Utf8PathBuf,
    command: Vec<OsString>,
    frequency: u32,
    call_graph: CallGraph,
    perf_data: Utf8PathBuf,
    output: Utf8PathBuf,
    title: String,
}

impl PerfInvocation {
    fn builder(root: &Utf8Path, command: Vec<OsString>) -> PerfInvocationBuilder {
        PerfInvocationBuilder {
            working_dir: root.to_owned(),
            command,
            frequency: 99,
            call_graph: CallGraph::Dwarf,
            perf_data: default_perf_data_path(root),
            output: default_output_path(root, &None, "flamegraph.svg"),
            title: "flamegraph".to_owned(),
        }
    }
}

struct PerfInvocationBuilder {
    working_dir: Utf8PathBuf,
    command: Vec<OsString>,
    frequency: u32,
    call_graph: CallGraph,
    perf_data: Utf8PathBuf,
    output: Utf8PathBuf,
    title: String,
}

impl PerfInvocationBuilder {
    fn frequency(mut self, value: u32) -> Self {
        self.frequency = value;
        self
    }

    fn call_graph(mut self, value: CallGraph) -> Self {
        self.call_graph = value;
        self
    }

    fn perf_data(mut self, value: Utf8PathBuf) -> Self {
        self.perf_data = value;
        self
    }

    fn output(mut self, value: Utf8PathBuf) -> Self {
        self.output = value;
        self
    }

    fn title(mut self, value: String) -> Self {
        self.title = value;
        self
    }

    fn build(self) -> PerfInvocation {
        PerfInvocation {
            working_dir: self.working_dir,
            command: self.command,
            frequency: self.frequency,
            call_graph: self.call_graph,
            perf_data: self.perf_data,
            output: self.output,
            title: self.title,
        }
    }
}

fn execute_perf(invocation: PerfInvocation) -> Result<()> {
    let PerfInvocation {
        working_dir,
        command,
        frequency,
        call_graph,
        perf_data,
        output,
        title,
    } = invocation;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent.as_std_path())?;
    }
    if let Some(parent) = perf_data.parent() {
        fs::create_dir_all(parent.as_std_path())?;
    }

    let perf_path: PathBuf = perf_data.clone().into();
    if perf_path.exists() {
        fs::remove_file(&perf_path)?;
    }

    let mut perf = Command::new("perf");
    perf.current_dir(working_dir.as_std_path());
    perf.arg("record");
    perf.arg("-F");
    perf.arg(frequency.to_string());
    perf.arg("-g");
    perf.arg("--call-graph");
    perf.arg(call_graph.as_flag());
    perf.arg("-o");
    perf.arg(&perf_path);
    perf.arg("--");
    let mut iter = command.iter();
    if let Some(program) = iter.next() {
        perf.arg(program);
    }
    for arg in iter {
        perf.arg(arg);
    }

    let command_preview = format_command(&command);
    println!("[perf] recording {command_preview}");
    run_command(perf).context("perf record failed")?;

    let mut perf_script = Command::new("perf");
    perf_script.current_dir(working_dir.as_std_path());
    perf_script.args(["script", "-i"]);
    perf_script.arg(&perf_path);
    perf_script.stdout(Stdio::piped());

    let mut child = perf_script
        .spawn()
        .context("failed to spawn `perf script`")?;
    let stdout = child
        .stdout
        .take()
        .context("failed to capture perf script output")?;

    let mut folder = PerfFolder::default();
    let mut collapsed = Vec::new();
    folder
        .collapse(BufReader::new(stdout), &mut collapsed)
        .context("failed to collapse perf samples")?;

    let status = child.wait()?;
    if !status.success() {
        bail!("`perf script` exited with status {status}");
    }

    let mut options = Options::default();
    options.count_name = "samples".to_owned();
    options.hash = true;
    options.title = title;

    let mut file = File::create(output.as_std_path())?;
    from_reader(&mut options, Cursor::new(collapsed), &mut file)
        .context("failed to render flamegraph")?;

    println!("Flamegraph written to {}", output);

    Ok(())
}

fn run_command(mut command: Command) -> Result<()> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .with_context(|| format!("failed to spawn `{program}`"))?;
    if !status.success() {
        bail!("`{program}` exited with status {status}");
    }
    Ok(())
}

fn default_perf_data_path(root: &Utf8Path) -> Utf8PathBuf {
    let dir = root.join("target/flamegraphs");
    Utf8PathBuf::from_path_buf(dir.join("capture.perf.data").into_std_path_buf())
        .expect("valid UTF-8 path")
}

fn format_command(args: &[OsString]) -> String {
    args.iter()
        .map(|arg| {
            let value = arg.to_string_lossy();
            if value.contains(' ') {
                format!("\"{value}\"")
            } else {
                value.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn open_file(path: &Utf8Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let mut command = Command::new("xdg-open");
        command.arg(path.as_std_path());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
        run_command(command)?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg(path.as_std_path());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
        run_command(command)?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("cmd");
        command.arg("/C");
        command.arg("start");
        command.arg(path.as_std_path());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
        run_command(command)?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    {
        bail!("--open is not supported on this platform");
    }
}
