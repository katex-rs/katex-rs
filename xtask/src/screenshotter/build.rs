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

    let dist_exists = dist_css.exists() && dist_fonts.exists();
    if dist_exists {
        return Ok(());
    }

    match mode {
        BuildMode::Never => {
            bail!(
                "KaTeX dist assets missing at {}. Remove --build never or build them manually.",
                dist_dir
            );
        }
        BuildMode::Always | BuildMode::Auto => {
            ensure_command_available("yarn")?;
            let status = Command::new("yarn")
                .arg("build")
                .current_dir(katex_dir.as_std_path())
                .status()
                .context("failed to run yarn build")?;
            if !status.success() {
                bail!("yarn build failed with status {status}");
            }
        }
    }

    if !dist_css.exists() || !dist_fonts.exists() {
        bail!(
            "KaTeX dist assets still missing after build at {}",
            dist_dir
        );
    }

    Ok(())
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
