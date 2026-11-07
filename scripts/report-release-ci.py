#!/usr/bin/env python3
"""Send an allowlisted Release workflow_run's job timings to Sentry Logs.

Requires GITHUB_EVENT_PATH, GH_TOKEN (actions:read), and CI_SENTRY_DSN.
Only workflow_run.completed events for Release are accepted. Only jobs started in the completed
attempt are reported. Replaying an attempt creates new records. Original job and
step times are recorded alongside the ingestion timestamp.
"""
import datetime as dt
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

# Exact matching keeps job and step text from becoming arbitrary telemetry.
JOBS = {"validate": ("validate", "all"), "assemble": ("assemble", "all"),
        "draft": ("draft", "all")}
for runner, target, bundles, asset in (
    ("macos-15", "aarch64-apple-darwin", "app,dmg", "darwin-aarch64"),
    ("macos-15-intel", "x86_64-apple-darwin", "app,dmg", "darwin-x86_64"),
    ("windows-2022", "x86_64-pc-windows-msvc", "nsis", "windows-x86_64"),
):
    JOBS[f"build ({runner}, {target}, {bundles}, {asset})"] = ("build", asset)
STEPS = {
    "Run actions/checkout@v4": "checkout",
    "Run actions/setup-node@v4": "setup_node",
    "Post Run actions/setup-node@v4": "post_setup_node",
    "Post Restore Rust dependency cache": "rust_cache_save",
    "Run actions/upload-artifact@v4": "upload_artifact",
    "Run actions/download-artifact@v4": "download_artifact",
    "Check versions and tag": "version_check",
    "Install Rust tools": "rust_tools",
    "Restore Rust dependency cache": "rust_cache_restore",
    "Rust cache exact hit": "rust_cache_exact_hit",
    "Run npm ci": "npm_install",
    "Run cargo fmt --manifest-path src-tauri/Cargo.toml -- --check": "rustfmt",
    "Run cargo test --locked --manifest-path src-tauri/Cargo.toml": "rust_test",
    "Run npm run test:ui": "ui_test",
    "Run cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings": "clippy",
    "Build installers": "build_installers",
    "Collect installers": "collect_installers",
    "Validate signed updater assets, generate manifest and checksums": "validate_assets",
    "Upload and verify draft release": "draft_release",
}
STATUSES = {"success", "failure", "cancelled", "skipped", "timed_out", "action_required", "neutral", "stale", "startup_failure"}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


OPENER = urllib.request.build_opener(NoRedirect())


def request_json(url, token):
    req = urllib.request.Request(url, headers={"Authorization": "Bearer " + token,
        "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "agent-gibbon-release-ci-reporter"})
    with OPENER.open(req, timeout=30) as response:
        return json.load(response)


def get_jobs(repo, run_id, attempt, token):
    jobs = []
    for page in range(1, 101):
        url = (f"https://api.github.com/repos/{repo}/actions/runs/{run_id}"
               f"/attempts/{attempt}/jobs?per_page=100&page={page}")
        result = request_json(url, token)
        batch = result.get("jobs")
        if not isinstance(batch, list):
            raise ValueError("invalid jobs response")
        jobs.extend(batch)
        if len(batch) < 100:
            return jobs
    raise ValueError("jobs pagination limit exceeded")


def timestamp(value):
    if not isinstance(value, str) or len(value) > 40:
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            return None
        return parsed.astimezone(dt.timezone.utc).isoformat().replace("+00:00", "Z")
    except ValueError:
        return None


def duration(start, end):
    if not start or not end:
        return None
    a = dt.datetime.fromisoformat(start.replace("Z", "+00:00"))
    b = dt.datetime.fromisoformat(end.replace("Z", "+00:00"))
    ms = int((b - a).total_seconds() * 1000)
    return ms if ms >= 0 else None


def conclusion(value):
    return value if value in STATUSES else "unknown"


def attrs(values):
    result = {}
    for key, value in values.items():
        if value is None:
            continue
        kind = "boolean" if isinstance(value, bool) else "integer" if isinstance(value, int) else "string"
        result[key] = {"type": kind, "value": value}
    return result


def logs_for_attempt(jobs, run_id, attempt, sha, trace_id):
    logs = []
    now = time.time()  # Historical events must not be discarded as stale.
    for job in jobs:
        if not isinstance(job, dict) or job.get("name") not in JOBS:
            continue
        label, platform = JOBS[job["name"]]
        steps = job.get("steps") if isinstance(job.get("steps"), list) else []
        cache_steps = [s for s in steps if isinstance(s, dict) and s.get("name") == "Rust cache exact hit"]
        cache = "unknown"
        if len(cache_steps) == 1:
            cache_status = cache_steps[0].get("conclusion")
            if cache_status == "success":
                cache = "true"
            elif cache_status == "skipped" and any(
                isinstance(s, dict) and s.get("name") == "Restore Rust dependency cache"
                and s.get("conclusion") == "success" for s in steps
            ):
                cache = "false"
        common = {"ci.run_id": str(run_id), "ci.attempt": attempt, "ci.sha": sha,
                  "ci.job": label, "ci.platform": platform, "ci.rust_cache_exact_hit": cache}
        records = [("job", label, job)] + [("step", STEPS[s["name"]], s) for s in steps
            if isinstance(s, dict) and s.get("name") in STEPS]
        for kind, name, record in records:
            start = timestamp(record.get("started_at"))
            end = timestamp(record.get("completed_at"))
            values = dict(common, **{"ci.kind": kind, "ci.name": name,
                "ci.status": conclusion(record.get("conclusion")),
                "ci.started_at": start, "ci.completed_at": end,
                "ci.duration_ms": duration(start, end), "sentry.environment": "ci"})
            logs.append({"timestamp": now, "trace_id": trace_id, "level": "info",
                         "body": "release_ci_timing", "attributes": attrs(values)})
    return logs


def send_logs(dsn, logs):
    parsed = urllib.parse.urlsplit(dsn)
    parts = parsed.path.strip("/").split("/")
    if (parsed.scheme != "https" or not parsed.hostname or not parsed.username or parsed.password
            or not re.fullmatch(r"[A-Za-z0-9]+", parsed.username)
            or not parts[-1].isdigit() or parsed.query or parsed.fragment):
        raise ValueError("invalid Sentry DSN")
    prefix = "/".join(parts[:-1])
    path = f"/{prefix + '/' if prefix else ''}api/{parts[-1]}/envelope/"
    host = parsed.hostname + (f":{parsed.port}" if parsed.port else "")
    url = urllib.parse.urlunsplit(("https", host, path, "", ""))
    header = {"type": "log", "item_count": len(logs), "content_type": "application/vnd.sentry.items.log+json"}
    envelope = "\n".join(json.dumps(x, separators=(",", ":")) for x in ({}, header, {"version": 2, "items": logs})) + "\n"
    req = urllib.request.Request(url, data=envelope.encode(), method="POST", headers={
        "Content-Type": "application/x-sentry-envelope",
        "X-Sentry-Auth": f"Sentry sentry_version=7, sentry_key={parsed.username}, sentry_client=agent-gibbon-release-ci/1.0"})
    with OPENER.open(req, timeout=30) as response:
        if not 200 <= response.status < 300:
            raise ValueError("Sentry rejected envelope")


def main():
    path = os.environ.get("GITHUB_EVENT_PATH")
    token = os.environ.get("GH_TOKEN")
    dsn = os.environ.get("CI_SENTRY_DSN")
    if not path or not token or not dsn:
        raise ValueError("GITHUB_EVENT_PATH, GH_TOKEN and CI_SENTRY_DSN are required")
    with open(path, encoding="utf-8") as event_file:
        event = json.load(event_file)
    run = event.get("workflow_run") or {}
    repo = (event.get("repository") or {}).get("full_name", "")
    run_id, attempt, sha = run.get("id"), run.get("run_attempt"), run.get("head_sha")
    if (event.get("action") != "completed" or run.get("name") != "Release"
            or run.get("status") != "completed" or not isinstance(repo, str)
            or not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo)
            or type(run_id) is not int or run_id < 1 or type(attempt) is not int or not 1 <= attempt <= 100
            or not isinstance(sha, str) or not re.fullmatch(r"[0-9a-fA-F]{40}", sha)):
        raise ValueError("event is not a valid completed Release workflow run")
    jobs = get_jobs(repo, run_id, attempt, token)
    start = timestamp(run.get("run_started_at"))
    end = timestamp(run.get("updated_at"))
    if attempt > 1:
        # GitHub includes previously successful jobs with new IDs and the new
        # attempt number, but preserves their original start and end times.
        if start is None:
            raise ValueError("retry start time missing")
        jobs = [job for job in jobs if isinstance(job, dict)
                and duration(start, timestamp(job.get("started_at"))) is not None]
    trace_id = uuid.uuid4().hex
    logs = logs_for_attempt(jobs, run_id, attempt, sha.lower(), trace_id)
    values = {"sentry.environment": "ci", "ci.run_id": str(run_id), "ci.attempt": attempt,
              "ci.sha": sha.lower(), "ci.kind": "workflow", "ci.name": "release",
              "ci.status": conclusion(run.get("conclusion")), "ci.started_at": start,
              "ci.completed_at": end, "ci.duration_ms": duration(start, end)}
    logs.insert(0, {"timestamp": time.time(), "trace_id": trace_id,
                    "level": "info", "body": "release_ci_timing", "attributes": attrs(values)})
    send_logs(dsn, logs)
    print(f"Sent {len(logs)} allowlisted records for run {run_id}, attempt {attempt}.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, urllib.error.URLError, json.JSONDecodeError):
        # Do not print response bodies, credentials, URLs, or arbitrary event data.
        print("Release CI report failed; check event, credentials and service availability.", file=sys.stderr)
        sys.exit(1)
