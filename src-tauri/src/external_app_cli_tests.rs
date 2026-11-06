//! Public local CLI contract; no native window or desktop is touched here.
use super::parse;

fn request(args: &[&str]) -> Result<serde_json::Value, String> {
    parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .and_then(|r| serde_json::to_value(r).map_err(|e| e.to_string()))
}

#[test]
fn external_app_cli_lists_without_note_side_effects() {
    for operation in ["list", "windows", "sessions", "select"] {
        let r = request(&["app", operation]).expect("external app command must parse");
        assert_eq!(r["action"], "app");
        assert_eq!(r["external"]["operation"], operation);
        assert!(r["markdown"].is_null());
        assert!(r["source_path"].is_null());
    }
}

#[test]
fn external_app_cli_preserves_physical_negative_desktop_region() {
    let r = request(&[
        "app",
        "open",
        "--app-id",
        "app-17",
        "--region",
        "-1600,80,640,480",
    ])
    .expect("installed app selection and physical region must parse");
    assert_eq!(r["external"]["app_id"], "app-17");
    assert_eq!(
        r["external"]["region"],
        serde_json::json!([-1600, 80, 640, 480])
    );
    let r = request(&[
        "app",
        "attach",
        "--window",
        "12345",
        "--region",
        "0,0,640,480",
    ])
    .expect("explicit existing window selection must parse");
    assert_eq!(r["external"]["window"], "12345");
    let r = request(&["app", "release", "--session", "floating-17"])
        .expect("release must address an owned session");
    assert_eq!(r["external"]["session"], "floating-17");
    assert!(request(&["app", "cancel", "--session", "launch-17"]).is_ok());
}

#[test]
fn external_app_cli_rejects_ambiguous_destructive_and_invalid_requests() {
    for args in [
        vec!["app", "open"],
        vec!["app", "attach", "--window", "12345"],
        vec!["app", "release"],
        vec!["app", "list", "--markdown", "data"],
        vec!["app", "open", "--app-id", "a", "--region", "0,0,0,80"],
        vec!["app", "open", "--app-id", "a", "--region", "NaN,0,600,400"],
        vec![
            "app",
            "open",
            "--app-id",
            "a",
            "--region",
            "0,0,600,400",
            "--app-id",
            "b",
        ],
        vec!["app", "attach", "--window", "0", "--region", "0,0,600,400"],
        vec!["app", "kill", "--window", "12345"],
        vec!["app", "list", "--session", "other"],
    ] {
        assert!(request(&args).is_err(), "must reject {args:?}");
    }
}
