use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde::Serialize;

use crate::error::{DoctorError, Result};
use crate::runmeta::RunMeta;
use crate::util::{OutputIntegration, output_for, relative_to, write_string};

#[derive(Debug, Clone, Args)]
pub struct ReportArgs {
    #[arg(long = "suite-dir")]
    pub suite_dir: PathBuf,

    #[arg(long = "output-html")]
    pub output_html: Option<PathBuf>,

    #[arg(long = "output-json")]
    pub output_json: Option<PathBuf>,

    #[arg(long, default_value = "TUI Inspector Report")]
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportSummary {
    pub title: String,
    pub suite_dir: String,
    pub generated_at: String,
    pub total_runs: usize,
    pub ok_runs: usize,
    pub failed_runs: usize,
    pub runs: Vec<RunMeta>,
}

fn find_run_meta_files(suite_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(suite_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let run_meta = entry.path().join("run_meta.json");
        if run_meta.exists() {
            files.push(run_meta);
        }
    }

    files.sort();
    Ok(files)
}

fn html_escape(value: &str) -> String {
    v_htmlescape::escape(value).to_string()
}

fn push_optional_artifact_link(
    html: &mut String,
    suite_dir: &Path,
    label: &str,
    path_value: Option<&str>,
) {
    let Some(path_value) = path_value.filter(|value| !value.is_empty()) else {
        return;
    };
    let path = PathBuf::from(path_value);
    if !path.exists() {
        return;
    }

    let rel = relative_to(suite_dir, &path).unwrap_or(path.clone());
    html.push_str(&format!(
        "<div class=\"row\"><span class=\"label\">{}</span><a href=\"{}\">{}</a></div>\n",
        html_escape(label),
        html_escape(&rel.display().to_string()),
        html_escape(&rel.display().to_string())
    ));
}

fn render_html(summary: &ReportSummary, suite_dir: &Path) -> String {
    let mut html = String::new();

    html.push_str(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
    );
    html.push_str(&format!(
        "  <title>{}</title>\n",
        html_escape(&summary.title)
    ));
    html.push_str(
        "  <style>\n    body { font-family: ui-sans-serif, -apple-system, Segoe UI, Roboto, Arial, sans-serif; margin: 24px; background: #0f1115; color: #e7ebf3; }\n    h1, h2 { margin: 0 0 12px; }\n    .meta { margin-bottom: 20px; color: #a8b0c5; }\n    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(380px, 1fr)); gap: 16px; }\n    .card { border: 1px solid #2a3142; border-radius: 10px; padding: 14px; background: #171b24; }\n    .ok { border-left: 5px solid #2cb67d; }\n    .fail { border-left: 5px solid #ef4565; }\n    .row { margin: 4px 0; font-size: 13px; color: #c8d0e3; }\n    .label { color: #8a95b5; display: inline-block; min-width: 130px; }\n    .snapshot { width: 100%; border-radius: 8px; border: 1px solid #2a3142; margin-top: 8px; }\n    video { width: 100%; margin-top: 8px; border-radius: 8px; border: 1px solid #2a3142; background: #090b10; }\n    a { color: #7da6ff; text-decoration: none; }\n    a:hover { text-decoration: underline; }\n    .pill { font-size: 11px; border: 1px solid #3a4460; border-radius: 999px; padding: 2px 8px; margin-left: 8px; color: #b9c6ee; }\n  </style>\n</head>\n<body>\n",
    );

    html.push_str(&format!("<h1>{}</h1>\n", html_escape(&summary.title)));
    html.push_str(&format!(
        "<div class=\"meta\">generated_at={} | total={} | ok={} | failed={}</div>\n",
        html_escape(&summary.generated_at),
        summary.total_runs,
        summary.ok_runs,
        summary.failed_runs
    ));
    html.push_str("<div class=\"grid\">\n");

    for run in &summary.runs {
        let status = run.status.as_str();
        let class_name = if status == "ok" { "ok" } else { "fail" };
        let run_path = PathBuf::from(&run.run_dir);
        let run_name = run_path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());

        let output_path = PathBuf::from(&run.output);
        let snapshot_path = PathBuf::from(&run.snapshot);

        let output_rel = relative_to(suite_dir, &output_path).unwrap_or(output_path.clone());
        let snapshot_rel = relative_to(suite_dir, &snapshot_path).unwrap_or(snapshot_path.clone());

        let output_rel_str = output_rel.display().to_string();
        let snapshot_rel_str = snapshot_rel.display().to_string();

        html.push_str(&format!("<section class=\"card {}\">\n", class_name));
        html.push_str(&format!(
            "<h2>{} <span class=\"pill\">{}</span></h2>\n",
            html_escape(&run.profile),
            html_escape(status)
        ));
        html.push_str(&format!(
            "<div class=\"row\"><span class=\"label\">run</span>{}</div>\n",
            html_escape(&run_name)
        ));
        html.push_str(&format!(
            "<div class=\"row\"><span class=\"label\">duration_seconds</span>{}</div>\n",
            run.duration_seconds
                .map_or_else(|| "null".to_string(), |value| value.to_string())
        ));
        html.push_str(&format!(
            "<div class=\"row\"><span class=\"label\">seed_demo</span>{}</div>\n",
            run.seed_demo
        ));
        html.push_str(&format!(
            "<div class=\"row\"><span class=\"label\">seed_exit_code</span>{}</div>\n",
            run.seed_exit_code
                .map_or_else(|| "null".to_string(), |value| value.to_string())
        ));
        html.push_str(&format!(
            "<div class=\"row\"><span class=\"label\">snapshot_status</span>{}</div>\n",
            html_escape(run.snapshot_status.as_deref().unwrap_or("unknown"))
        ));
        html.push_str(&format!(
            "<div class=\"row\"><span class=\"label\">vhs_exit_code</span>{}</div>\n",
            run.vhs_exit_code
                .map_or_else(|| "null".to_string(), |value| value.to_string())
        ));
        if let Some(trace_id) = run.trace_id.as_deref().filter(|value| !value.is_empty()) {
            html.push_str(&format!(
                "<div class=\"row\"><span class=\"label\">trace_id</span>{}</div>\n",
                html_escape(trace_id)
            ));
        }
        if let Some(fallback_reason) = run
            .fallback_reason
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            html.push_str(&format!(
                "<div class=\"row\"><span class=\"label\">fallback_reason</span>{}</div>\n",
                html_escape(fallback_reason)
            ));
        }
        push_optional_artifact_link(
            &mut html,
            suite_dir,
            "evidence_ledger",
            run.evidence_ledger.as_deref(),
        );
        push_optional_artifact_link(
            &mut html,
            suite_dir,
            "ttyd_shim_log",
            run.ttyd_shim_log.as_deref(),
        );
        push_optional_artifact_link(
            &mut html,
            suite_dir,
            "ttyd_runtime_log",
            run.ttyd_runtime_log.as_deref(),
        );
        push_optional_artifact_link(
            &mut html,
            suite_dir,
            "vhs_docker_log",
            run.vhs_docker_log.as_deref(),
        );

        if !run.output.is_empty() && Path::new(&run.output).exists() {
            html.push_str(&format!(
                "<div class=\"row\"><a href=\"{}\">video file</a></div>\n",
                html_escape(&output_rel_str)
            ));
            html.push_str(&format!(
                "<video controls muted preload=\"metadata\" src=\"{}\"></video>\n",
                html_escape(&output_rel_str)
            ));
        }

        if !run.snapshot.is_empty() && Path::new(&run.snapshot).exists() {
            html.push_str(&format!(
                "<div class=\"row\"><a href=\"{}\">snapshot file</a></div>\n",
                html_escape(&snapshot_rel_str)
            ));
            html.push_str(&format!(
                "<img class=\"snapshot\" alt=\"snapshot {}\" src=\"{}\">\n",
                html_escape(&run.profile),
                html_escape(&snapshot_rel_str)
            ));
        }

        html.push_str("</section>\n");
    }

    html.push_str("</div>\n</body>\n</html>\n");
    html
}

pub fn run_report(args: ReportArgs) -> Result<()> {
    let integration = OutputIntegration::detect();
    run_report_with_integration(args, &integration)
}

fn run_report_with_integration(args: ReportArgs, integration: &OutputIntegration) -> Result<()> {
    let ui = output_for(integration);

    if !args.suite_dir.exists() {
        return Err(DoctorError::MissingPath {
            path: args.suite_dir,
        });
    }

    let output_html = args
        .output_html
        .unwrap_or_else(|| args.suite_dir.join("index.html"));
    let output_json = args
        .output_json
        .unwrap_or_else(|| args.suite_dir.join("report.json"));

    let meta_files = find_run_meta_files(&args.suite_dir)?;
    if meta_files.is_empty() {
        return Err(DoctorError::invalid(format!(
            "No run_meta.json files found under {}",
            args.suite_dir.display()
        )));
    }

    let runs = meta_files
        .iter()
        .map(|path| RunMeta::from_path(path))
        .collect::<Result<Vec<_>>>()?;

    let ok_runs = runs.iter().filter(|run| run.status == "ok").count();
    let failed_runs = runs.len().saturating_sub(ok_runs);

    let summary = ReportSummary {
        title: args.title,
        suite_dir: args.suite_dir.display().to_string(),
        generated_at: crate::util::now_utc_iso(),
        total_runs: runs.len(),
        ok_runs,
        failed_runs,
        runs,
    };

    let json_content = serde_json::to_string_pretty(&summary)?;
    write_string(&output_json, &json_content)?;

    let html = render_html(&summary, &args.suite_dir);
    write_string(&output_html, &html)?;

    ui.success(&format!("report JSON: {}", output_json.display()));
    ui.success(&format!("report HTML: {}", output_html.display()));

    if integration.should_emit_json() {
        println!(
            "{}",
            serde_json::json!({
                "command": "report",
                "status": "ok",
                "report_json": output_json.display().to_string(),
                "report_html": output_html.display().to_string(),
                "suite_dir": args.suite_dir.display().to_string(),
                "integration": integration,
            })
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::error::DoctorError;
    use crate::runmeta::RunMeta;
    use crate::util::OutputIntegration;

    use super::{ReportArgs, find_run_meta_files, run_report, run_report_with_integration};

    #[test]
    fn report_generation_writes_outputs() {
        let temp = tempdir().expect("tempdir");
        let suite_dir = temp.path().join("suite");
        let run_dir = suite_dir.join("run_01");
        std::fs::create_dir_all(&run_dir).expect("mkdir");

        let output_path = run_dir.join("capture.mp4");
        let snapshot_path = run_dir.join("snapshot.png");
        fs::write(&output_path, b"dummy").expect("write dummy video");
        fs::write(&snapshot_path, b"dummy").expect("write dummy snapshot");

        let run_meta = RunMeta {
            status: "ok".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "analytics-empty".to_string(),
            output: output_path.display().to_string(),
            snapshot: snapshot_path.display().to_string(),
            run_dir: run_dir.display().to_string(),
            ..RunMeta::default()
        };

        run_meta
            .write_to_path(&run_dir.join("run_meta.json"))
            .expect("write run_meta");

        let args = ReportArgs {
            suite_dir: suite_dir.clone(),
            output_html: None,
            output_json: None,
            title: "Report".to_string(),
        };

        run_report(args).expect("run report");

        assert!(suite_dir.join("index.html").exists());
        assert!(suite_dir.join("report.json").exists());
    }

    #[test]
    fn find_run_meta_files_sorts_and_skips_non_directories() {
        let temp = tempdir().expect("tempdir");
        let suite_dir = temp.path().join("suite");
        let a_run = suite_dir.join("a_run");
        let b_run = suite_dir.join("b_run");
        let c_run = suite_dir.join("c_run");
        fs::create_dir_all(&a_run).expect("mkdir a_run");
        fs::create_dir_all(&b_run).expect("mkdir b_run");
        fs::create_dir_all(&c_run).expect("mkdir c_run");
        fs::write(suite_dir.join("not_a_dir"), b"ignore").expect("write file");

        RunMeta {
            status: "ok".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "alpha".to_string(),
            output: a_run.join("capture.mp4").display().to_string(),
            run_dir: a_run.display().to_string(),
            ..RunMeta::default()
        }
        .write_to_path(&a_run.join("run_meta.json"))
        .expect("write a run meta");

        RunMeta {
            status: "failed".to_string(),
            started_at: "2026-02-17T00:00:01Z".to_string(),
            profile: "beta".to_string(),
            output: b_run.join("capture.mp4").display().to_string(),
            run_dir: b_run.display().to_string(),
            ..RunMeta::default()
        }
        .write_to_path(&b_run.join("run_meta.json"))
        .expect("write b run meta");

        let files = find_run_meta_files(&suite_dir).expect("find run meta files");
        let display = files
            .iter()
            .map(|path| {
                path.strip_prefix(&suite_dir)
                    .unwrap_or(path)
                    .display()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(display, vec!["a_run/run_meta.json", "b_run/run_meta.json"]);
    }

    #[test]
    fn run_report_fails_when_suite_dir_missing() {
        let temp = tempdir().expect("tempdir");
        let missing_suite_dir = temp.path().join("does_not_exist");

        let error = run_report(ReportArgs {
            suite_dir: missing_suite_dir.clone(),
            output_html: None,
            output_json: None,
            title: "Report".to_string(),
        })
        .expect_err("missing suite dir should fail");

        assert!(matches!(&error, DoctorError::MissingPath { .. }));
        if let DoctorError::MissingPath { path } = error {
            assert_eq!(path, missing_suite_dir);
        }
    }

    #[test]
    fn run_report_fails_when_no_run_meta_files_present() {
        let temp = tempdir().expect("tempdir");
        let suite_dir = temp.path().join("suite");
        fs::create_dir_all(&suite_dir).expect("mkdir suite");

        let error = run_report(ReportArgs {
            suite_dir: suite_dir.clone(),
            output_html: None,
            output_json: None,
            title: "Report".to_string(),
        })
        .expect_err("suite dir without run meta files should fail");
        assert!(
            error
                .to_string()
                .contains("No run_meta.json files found under"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn run_report_respects_output_path_overrides() {
        let temp = tempdir().expect("tempdir");
        let suite_dir = temp.path().join("suite");
        let run_dir = suite_dir.join("run_01");
        fs::create_dir_all(&run_dir).expect("mkdir");

        let run_meta = RunMeta {
            status: "ok".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "analytics-empty".to_string(),
            output: run_dir.join("capture.mp4").display().to_string(),
            run_dir: run_dir.display().to_string(),
            ..RunMeta::default()
        };
        run_meta
            .write_to_path(&run_dir.join("run_meta.json"))
            .expect("write run meta");

        let output_html = temp.path().join("custom.html");
        let output_json = temp.path().join("custom.json");

        run_report(ReportArgs {
            suite_dir: suite_dir.clone(),
            output_html: Some(output_html.clone()),
            output_json: Some(output_json.clone()),
            title: "Custom Report".to_string(),
        })
        .expect("run report");

        assert!(output_html.exists());
        assert!(output_json.exists());

        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output_json).expect("read json"))
                .expect("parse json");
        assert_eq!(parsed["title"], "Custom Report");
        assert_eq!(parsed["suite_dir"], suite_dir.display().to_string());
        assert_eq!(parsed["total_runs"], 1);
        assert_eq!(parsed["ok_runs"], 1);
        assert_eq!(parsed["failed_runs"], 0);
    }

    #[test]
    fn run_report_escapes_html_title() {
        let temp = tempdir().expect("tempdir");
        let suite_dir = temp.path().join("suite");
        let run_dir = suite_dir.join("run_01");
        fs::create_dir_all(&run_dir).expect("mkdir");

        let video_path = run_dir.join("capture.mp4");
        fs::write(&video_path, b"not-a-real-mp4").expect("write dummy video");

        let snapshot_path = run_dir.join("snapshot.png");
        fs::write(&snapshot_path, b"not-a-real-png").expect("write dummy snapshot");

        let run_meta = RunMeta {
            status: "ok".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "analytics-empty".to_string(),
            output: video_path.display().to_string(),
            snapshot: snapshot_path.display().to_string(),
            run_dir: run_dir.display().to_string(),
            ..RunMeta::default()
        };
        run_meta
            .write_to_path(&run_dir.join("run_meta.json"))
            .expect("write run meta");

        let title = "Report <script>alert(1)</script>";
        run_report(ReportArgs {
            suite_dir: suite_dir.clone(),
            output_html: None,
            output_json: None,
            title: title.to_string(),
        })
        .expect("run report");

        let html = fs::read_to_string(suite_dir.join("index.html")).expect("read html");
        assert!(
            html.contains("Report &lt;script&gt;alert(1)&lt;&#x2f;script&gt;"),
            "expected escaped title, got: {html}"
        );
        assert!(!html.contains("<script>alert(1)</script>"));

        // Ensure the file-exists conditionals emit links when the artifacts exist.
        assert!(html.contains("video file"));
        assert!(html.contains("snapshot file"));
    }

    #[test]
    fn run_report_html_surfaces_evidence_links_and_fallback_metadata() {
        let temp = tempdir().expect("tempdir");
        let suite_dir = temp.path().join("suite");
        let run_dir = suite_dir.join("run_01");
        fs::create_dir_all(&run_dir).expect("mkdir");

        let evidence_ledger = run_dir.join("evidence_ledger.jsonl");
        let ttyd_runtime_log = run_dir.join("ttyd-runtime.log");
        fs::write(&evidence_ledger, b"{}\n").expect("write ledger");
        fs::write(&ttyd_runtime_log, b"log").expect("write runtime log");

        RunMeta {
            status: "degraded".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "analytics-empty".to_string(),
            run_dir: run_dir.display().to_string(),
            trace_id: Some("trace-123".to_string()),
            fallback_reason: Some("capture timed out".to_string()),
            evidence_ledger: Some(evidence_ledger.display().to_string()),
            ttyd_runtime_log: Some(ttyd_runtime_log.display().to_string()),
            ..RunMeta::default()
        }
        .write_to_path(&run_dir.join("run_meta.json"))
        .expect("write run meta");

        run_report(ReportArgs {
            suite_dir: suite_dir.clone(),
            output_html: None,
            output_json: None,
            title: "Artifact Report".to_string(),
        })
        .expect("run report");

        let html = fs::read_to_string(suite_dir.join("index.html")).expect("read html");
        assert!(html.contains("trace-123"));
        assert!(html.contains("capture timed out"));
        assert!(html.contains("evidence_ledger"));
        assert!(html.contains("evidence_ledger.jsonl"));
        assert!(html.contains("ttyd_runtime_log"));
        assert!(html.contains("ttyd-runtime.log"));
    }

    #[test]
    fn run_report_emits_machine_json_when_sqlmodel_json_enabled() {
        let temp = tempdir().expect("tempdir");
        let suite_dir = temp.path().join("suite");
        let run_dir = suite_dir.join("run_01");
        fs::create_dir_all(&run_dir).expect("mkdir");

        RunMeta {
            status: "ok".to_string(),
            started_at: "2026-02-17T00:00:00Z".to_string(),
            profile: "analytics-empty".to_string(),
            output: run_dir.join("capture.mp4").display().to_string(),
            run_dir: run_dir.display().to_string(),
            ..RunMeta::default()
        }
        .write_to_path(&run_dir.join("run_meta.json"))
        .expect("write run meta");

        let integration = OutputIntegration {
            fastapi_mode: "plain".to_string(),
            fastapi_agent: false,
            fastapi_ci: false,
            fastapi_tty: false,
            sqlmodel_mode: "json".to_string(),
            sqlmodel_agent: false,
        };
        run_report_with_integration(
            ReportArgs {
                suite_dir,
                output_html: None,
                output_json: None,
                title: "JSON Report".to_string(),
            },
            &integration,
        )
        .expect("report should succeed in json mode");
    }
}
