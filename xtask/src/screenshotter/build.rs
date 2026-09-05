use std::fs;
use std::process::Command;
use std::time::SystemTime;

use camino::Utf8Path;
use color_eyre::eyre::{Context, Result, bail};

use crate::screenshotter::args::BuildMode;

pub fn ensure_katex_dist_assets(root: &Utf8Path, mode: BuildMode) -> Result<()> {
    let katex_dir = root.join("KaTeX");
    let dist_dir = katex_dir.join("dist");
    let dist_css = dist_dir.join("katex.min.css");
    let dist_fonts = dist_dir.join("fonts");

    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&katex_dir)
        .output()
        .context("failed to read KaTeX revision")?;
    if !revision.status.success() {
        bail!("failed to read KaTeX revision");
    }
    let revision = String::from_utf8(revision.stdout)?.trim().to_owned();
    let stamp = dist_dir.join(".katex-rs-revision");
    let current = katex_dist_is_current(&dist_dir, &revision);
    match mode {
        BuildMode::Never if !current => bail!(
            "KaTeX dist assets missing or stale at {}. Run with --build auto or --build always.",
            dist_dir
        ),
        BuildMode::Never => return Ok(()),
        BuildMode::Auto if current => return Ok(()),
        BuildMode::Always | BuildMode::Auto => run_pnpm_build(&katex_dir)?,
    }
    if !dist_css.exists() || !dist_fonts.exists() || !dist_dir.join("katex.min.js").exists() {
        bail!(
            "KaTeX dist assets still missing after build at {}",
            dist_dir
        );
    }
    fs::write(&stamp, revision).context("failed to stamp KaTeX dist revision")?;
    Ok(())
}

fn katex_dist_is_current(dist_dir: &Utf8Path, revision: &str) -> bool {
    dist_dir.join("katex.min.css").is_file()
        && dist_dir.join("katex.min.js").is_file()
        && dist_dir.join("fonts").is_dir()
        && fs::read_to_string(dist_dir.join(".katex-rs-revision"))
            .is_ok_and(|stamp| stamp == revision)
}

pub fn ensure_wasm_artifacts(root: &Utf8Path, mode: BuildMode) -> Result<()> {
    let wasm_crate = root.join("crates/wasm-binding");
    let pkg_dir = wasm_crate.join("pkg");

    let mut need_build = false;
    match mode {
        BuildMode::Always => need_build = true,
        BuildMode::Never => {
            if pkg_dir.join("katex.js").exists() {
                need_build = false;
            } else {
                bail!(
                    "wasm-pack artifacts missing at {}. Remove --build never or build them manually.",
                    pkg_dir
                );
            }
        }
        BuildMode::Auto => {
            if !pkg_dir.join("katex.js").exists() {
                need_build = true;
            } else {
                let binding_src_meta = fs::metadata(wasm_crate.join("src").as_std_path())?;
                let katex_src_meta = fs::metadata(root.join("crates/katex/src").as_std_path())?;
                let pkg_meta = fs::metadata(pkg_dir.as_std_path())?;
                let binding_mtime = binding_src_meta
                    .modified()
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let katex_mtime = katex_src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let newest_src = binding_mtime.max(katex_mtime);
                let pkg_mtime = pkg_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                if newest_src > pkg_mtime {
                    need_build = true;
                }
            }
        }
    }

    if !need_build {
        return Ok(());
    }

    ensure_command_available("wasm-pack")?;

    let status = Command::new("wasm-pack")
        .args(["build", "--target", "web", "--no-opt", "--dev"])
        .current_dir(wasm_crate.as_std_path())
        .status()
        .context("failed to run wasm-pack build")?;
    if !status.success() {
        bail!("wasm-pack build failed with status {status}");
    }

    if !pkg_dir.join("katex.js").exists() {
        bail!(
            "wasm-pack build completed but {} is still missing",
            pkg_dir.join("katex.js")
        );
    }

    Ok(())
}

pub fn ensure_command_available(program: &str) -> Result<()> {
    let status = Command::new(program)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("command `{program}` exited with status {status}"),
        Err(err) => bail!("failed to execute `{program}`: {err}"),
    }
}

fn pnpm_build_command(katex_dir: &Utf8Path) -> Command {
    // Corepack selects the exact packageManager version pinned by upstream.
    // npm installs a .cmd shim on Windows; Command only infers .exe extensions.
    let program = if cfg!(windows) {
        "corepack.cmd"
    } else {
        "corepack"
    };
    let mut command = Command::new(program);
    command.args(["pnpm", "build"]).current_dir(katex_dir);
    command
}

fn run_pnpm_build(katex_dir: &Utf8Path) -> Result<()> {
    let status = pnpm_build_command(katex_dir)
        .status()
        .context("failed to run corepack pnpm build; install Corepack and run corepack pnpm install --frozen-lockfile in KaTeX")?;
    if !status.success() {
        bail!("pnpm build failed with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    #[test]
    fn pnpm_build_uses_platform_shim_and_upstream_directory() {
        let dir = Utf8Path::new("workspace with spaces/KaTeX");
        let command = pnpm_build_command(dir);
        assert_eq!(
            command.get_program(),
            if cfg!(windows) {
                "corepack.cmd"
            } else {
                "corepack"
            }
        );
        assert_eq!(command.get_args().collect::<Vec<_>>(), ["pnpm", "build"]);
        assert_eq!(command.get_current_dir(), Some(dir.as_std_path()));
    }

    #[cfg(windows)]
    #[test]
    fn pnpm_build_executes_windows_cmd_shim() -> Result<()> {
        let temp = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let root = Utf8PathBuf::from_path_buf(temp.join(format!(
            "katex corepack shim {}-{nonce}",
            std::process::id()
        )))
        .map_err(|_| color_eyre::eyre::eyre!("non-UTF8 temporary directory"))?;
        let katex_dir = root.join("KaTeX");
        fs::create_dir_all(&katex_dir)?;
        fs::write(
            root.join("corepack.cmd"),
            "@echo off\r\nif not \"%~1\"==\"pnpm\" exit /b 2\r\nif not \"%~2\"==\"build\" exit /b 3\r\necho built>shim-result.txt\r\nexit /b 0\r\n",
        )?;
        // Set PATH only for the child; parallel tests keep their environment.
        let status = pnpm_build_command(&katex_dir)
            .env("PATH", root.as_str())
            .status()?;
        let marker = fs::read_to_string(katex_dir.join("shim-result.txt"));
        assert_eq!(root.as_std_path().parent(), Some(temp.as_path()));
        fs::remove_dir_all(root)?;
        assert!(status.success());
        assert_eq!(marker?.trim(), "built");
        Ok(())
    }

    #[test]
    fn rejects_missing_and_stale_reference_assets() -> Result<()> {
        let temp = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let root = Utf8PathBuf::from_path_buf(
            temp.join(format!("katex-dist-{}-{nonce}", std::process::id())),
        )
        .map_err(|_| color_eyre::eyre::eyre!("non-UTF8 temporary directory"))?;
        fs::create_dir_all(root.join("fonts"))?;
        assert!(!katex_dist_is_current(&root, "new"));
        fs::write(root.join("katex.min.css"), "css")?;
        fs::write(root.join("katex.min.js"), "js")?;
        assert!(!katex_dist_is_current(&root, "new"));
        fs::write(root.join(".katex-rs-revision"), "old")?;
        assert!(!katex_dist_is_current(&root, "new"));
        fs::write(root.join(".katex-rs-revision"), "new")?;
        assert!(katex_dist_is_current(&root, "new"));
        fs::remove_file(root.join("katex.min.js"))?;
        assert!(!katex_dist_is_current(&root, "new"));
        // Only remove the unique directory created by this test.
        assert_eq!(root.as_std_path().parent(), Some(temp.as_path()));
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
