// SPDX-FileCopyrightText: 2026 Greenbone AG
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#[path = "../feed/mod.rs"]
mod feed;

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use feed::{
    alert_doc::parse_alert_doc,
    generator::{GenerateConfig, generate_vts},
    validation::ValidationReport,
};

#[derive(Debug)]
struct Cli {
    input: PathBuf,
    output: PathBuf,
    version_date: String,
    fail_on_warning: bool,
    dry_run: bool,
}

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<(), ExitCode> {
    let cli = match parse_cli(args) {
        Ok(Some(cli)) => cli,
        Ok(None) => return Ok(()),
        Err(message) => {
            eprintln!("error: {message}");
            print_usage();
            return Err(ExitCode::from(2));
        }
    };

    let (docs, mut report, input_count) = load_docs(&cli.input);
    let config = GenerateConfig {
        version_date: cli.version_date.clone(),
    };
    let (vts, generation_report) = generate_vts(docs, &config);
    report.extend(generation_report);

    report.print();

    if report.has_errors() {
        print_summary(input_count, 0, &report, &cli.output, cli.dry_run);
        return Err(ExitCode::FAILURE);
    }

    if !cli.dry_run
        && let Err(err) = fs::create_dir_all(&cli.output)
    {
        eprintln!(
            "error: failed to create output directory {}: {err}",
            cli.output.display()
        );
        return Err(ExitCode::FAILURE);
    }

    for vt in &vts {
        let path = cli.output.join(&vt.file_name);
        if cli.dry_run {
            println!("would write {}", path.display());
            continue;
        }
        if let Err(err) = fs::write(&path, &vt.rendered) {
            eprintln!("error: failed to write {}: {err}", path.display());
            return Err(ExitCode::FAILURE);
        }
    }

    print_summary(input_count, vts.len(), &report, &cli.output, cli.dry_run);
    if cli.fail_on_warning && report.has_warnings() {
        return Err(ExitCode::FAILURE);
    }
    Ok(())
}

fn parse_cli(args: impl IntoIterator<Item = String>) -> Result<Option<Cli>, String> {
    let mut input = None;
    let mut output = None;
    let mut version_date = None;
    let mut fail_on_warning = false;
    let mut dry_run = false;

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_usage();
                return Ok(None);
            }
            "--input" => input = Some(next_arg(&mut args, "--input")?.into()),
            "--output" => output = Some(next_arg(&mut args, "--output")?.into()),
            "--version-date" => version_date = Some(next_arg(&mut args, "--version-date")?),
            "--fail-on-warning" => fail_on_warning = true,
            "--dry-run" => dry_run = true,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    Ok(Some(Cli {
        input: input.ok_or("--input is required")?,
        output: output.ok_or("--output is required")?,
        version_date: version_date.unwrap_or_else(current_utc_version_date),
        fail_on_warning,
        dry_run,
    }))
}

fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_usage() {
    eprintln!(
        "usage: greenbone-was-vtgen --input <dir> --output <dir> [--version-date 2026-06-01T00:00:00+0000] [--fail-on-warning] [--dry-run]"
    );
}

fn load_docs(input: &Path) -> (Vec<feed::alert_doc::AlertDoc>, ValidationReport, usize) {
    let mut report = ValidationReport::default();
    let mut docs = Vec::new();
    let mut paths = Vec::new();

    collect_markdown_files(input, &mut paths, &mut report);
    paths.sort();
    let input_count = paths.len();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(content) => match parse_alert_doc(&path, &content) {
                Ok(Some(doc)) => docs.push(doc),
                Ok(None) => {}
                Err(err) => report.error(path, None, err),
            },
            Err(err) => report.error(path, None, format!("failed to read file: {err}")),
        }
    }

    (docs, report, input_count)
}

fn collect_markdown_files(input: &Path, paths: &mut Vec<PathBuf>, report: &mut ValidationReport) {
    let entries = match fs::read_dir(input) {
        Ok(entries) => entries,
        Err(err) => {
            report.error(
                input.to_path_buf(),
                None,
                format!("failed to read input directory: {err}"),
            );
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report.error(
                    input.to_path_buf(),
                    None,
                    format!("failed to read entry: {err}"),
                );
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, paths, report);
        } else if path.extension() == Some(OsStr::new("md"))
            && path.file_name() != Some(OsStr::new("_index.md"))
        {
            paths.push(path);
        }
    }
}

fn print_summary(
    input_count: usize,
    generated_count: usize,
    report: &ValidationReport,
    output: &Path,
    dry_run: bool,
) {
    let mode = if dry_run { "dry-run" } else { "write" };
    println!(
        "summary: mode={mode} inputs={input_count} generated={generated_count} warnings={} errors={} output={}",
        report.warning_count(),
        report.error_count(),
        output.display()
    );
}

fn current_utc_version_date() -> String {
    let seconds = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(_) => 0,
    };
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}+0000")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_requires_input_and_output_to_prevent_accidental_generation() {
        let err = parse_cli(["--input".to_string(), "alerts".to_string()]).unwrap_err();
        assert_eq!(err, "--output is required");
    }

    #[test]
    fn parse_cli_rejects_removed_contributor_option() {
        let err = parse_cli([
            "--input".to_string(),
            "alerts".to_string(),
            "--output".to_string(),
            "nasl".to_string(),
            "--contributor".to_string(),
            "123456".to_string(),
        ])
        .unwrap_err();
        assert_eq!(err, "unknown argument: --contributor");
    }

    #[test]
    fn civil_from_days_formats_unix_epoch_for_default_version_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }
}
