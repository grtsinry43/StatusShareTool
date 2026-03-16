use std::env;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use mpris::{PlaybackStatus, PlayerFinder};
use statusshare_core::{MediaInfo, WindowInfo};

#[derive(Debug, Clone)]
pub struct DetectedWindow {
    pub backend: String,
    pub window: WindowInfo,
}

pub fn detect_session_type() -> String {
    if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
        return session_type;
    }
    if env::var("WAYLAND_DISPLAY").is_ok() {
        return "wayland".to_string();
    }
    if env::var("DISPLAY").is_ok() {
        return "x11".to_string();
    }
    "unknown".to_string()
}

pub fn detect_active_window() -> Result<DetectedWindow, String> {
    let session_type = detect_session_type();
    let mut errors = Vec::new();

    match session_type.as_str() {
        "wayland" => {
            if is_kde_wayland() {
                match detect_active_window_wayland_kde() {
                    Ok(window) => {
                        return Ok(DetectedWindow {
                            backend: "kwin-wayland".to_string(),
                            window,
                        });
                    }
                    Err(err) => errors.push(format!("kwin-wayland: {err}")),
                }
            }

            match detect_active_window_with_hyprctl() {
                Ok(window) => {
                    return Ok(DetectedWindow {
                        backend: "hyprctl".to_string(),
                        window,
                    });
                }
                Err(err) => errors.push(format!("hyprctl: {err}")),
            }

            match detect_active_window_with_swaymsg() {
                Ok(window) => {
                    return Ok(DetectedWindow {
                        backend: "swaymsg".to_string(),
                        window,
                    });
                }
                Err(err) => errors.push(format!("swaymsg: {err}")),
            }

            if env::var("DISPLAY").is_ok() {
                match detect_active_window_x11() {
                    Ok(window) => {
                        return Ok(DetectedWindow {
                            backend: "x11-xprop".to_string(),
                            window,
                        });
                    }
                    Err(err) => errors.push(format!("x11-xprop: {err}")),
                }
            }
        }
        "x11" => match detect_active_window_x11() {
            Ok(window) => {
                return Ok(DetectedWindow {
                    backend: "x11-xprop".to_string(),
                    window,
                });
            }
            Err(err) => errors.push(format!("x11-xprop: {err}")),
        },
        _ => {
            for attempt in [
                detect_active_window_with_hyprctl as fn() -> Result<WindowInfo, String>,
                detect_active_window_with_swaymsg,
                detect_active_window_x11,
            ] {
                if let Ok(window) = attempt() {
                    return Ok(DetectedWindow {
                        backend: "fallback".to_string(),
                        window,
                    });
                }
            }
            errors.push("no compatible backend succeeded".to_string());
        }
    }

    match detect_active_window_with_xdotool() {
        Ok(window) => Ok(DetectedWindow {
            backend: "xdotool".to_string(),
            window,
        }),
        Err(err) => {
            errors.push(format!("xdotool: {err}"));
            Err(errors.join("\n"))
        }
    }
}

pub fn detect_media() -> Result<Option<MediaInfo>, String> {
    let finder = PlayerFinder::new().map_err(|err| err.to_string())?;
    let players = finder.find_all().map_err(|err| err.to_string())?;

    let mut first_with_metadata = None;

    for player in players {
        let status = player.get_playback_status().ok();
        let media = media_from_player(&player);

        if media.is_none() {
            continue;
        }

        if matches!(status, Some(PlaybackStatus::Playing)) {
            return Ok(media);
        }

        if first_with_metadata.is_none() {
            first_with_metadata = media;
        }
    }

    Ok(first_with_metadata)
}

fn media_from_player(player: &mpris::Player) -> Option<MediaInfo> {
    let metadata = player.get_metadata().ok()?;
    let title = metadata.title().unwrap_or_default().trim().to_string();
    let artist = metadata
        .artists()
        .map(|artists| artists.join(", "))
        .unwrap_or_default()
        .trim()
        .to_string();
    let thumbnail = metadata
        .art_url()
        .map(|url| url.to_string())
        .unwrap_or_default()
        .trim()
        .to_string();

    if title.is_empty() && artist.is_empty() && thumbnail.is_empty() {
        None
    } else {
        Some(MediaInfo {
            title,
            artist,
            thumbnail,
        })
    }
}

fn is_kde_wayland() -> bool {
    env::var("KDE_SESSION_VERSION").is_ok()
        || env::var("DESKTOP_SESSION")
            .unwrap_or_default()
            .to_lowercase()
            .contains("plasma")
        || env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase()
            .contains("kde")
}

fn detect_dbus_tool() -> Option<String> {
    ["qdbus6", "qdbus"]
        .into_iter()
        .find(|tool| Command::new(tool).arg("--version").output().is_ok())
        .map(ToString::to_string)
}

fn detect_active_window_wayland_kde() -> Result<WindowInfo, String> {
    let dbus_tool = detect_dbus_tool()
        .ok_or_else(|| "missing qdbus6/qdbus for KWin integration".to_string())?;

    let script_content = r#"
try {
    const activeWindow = workspace.activeWindow;
    if (activeWindow && activeWindow.resourceClass) {
        print("ACTIVE_WINDOW_CLASS:" + activeWindow.resourceClass.toString());
        print("ACTIVE_WINDOW_CAPTION:" + (activeWindow.caption || "").toString());
    } else {
        print("ACTIVE_WINDOW_CLASS:");
        print("ACTIVE_WINDOW_CAPTION:");
    }
} catch (e) {
    print("ERROR:" + e.toString());
}
"#;

    let script_path = env::temp_dir().join("statusshare_kwin_active_window.js");
    fs::write(&script_path, script_content).map_err(|err| err.to_string())?;
    let since_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_secs();

    let load_output = Command::new(&dbus_tool)
        .arg("org.kde.KWin")
        .arg("/Scripting")
        .arg("org.kde.kwin.Scripting.loadScript")
        .arg(script_path.to_string_lossy().as_ref())
        .output()
        .map_err(|err| err.to_string())?;

    if !load_output.status.success() {
        let _ = fs::remove_file(&script_path);
        return Err(String::from_utf8_lossy(&load_output.stderr)
            .trim()
            .to_string());
    }

    let script_id = String::from_utf8_lossy(&load_output.stdout)
        .trim()
        .to_string();
    if script_id.is_empty() {
        let _ = fs::remove_file(&script_path);
        return Err("kwin scripting returned empty script id".to_string());
    }

    let _ = Command::new(&dbus_tool)
        .arg("org.kde.KWin")
        .arg(format!("/Scripting/Script{script_id}"))
        .arg("org.kde.kwin.Script.run")
        .output();

    thread::sleep(Duration::from_millis(250));

    let journal_output = Command::new("journalctl")
        .arg("_COMM=kwin_wayland")
        .arg("-o")
        .arg("cat")
        .arg("--since")
        .arg(format!("@{since_secs}"))
        .arg("--no-pager")
        .output()
        .map_err(|err| err.to_string())?;

    let _ = Command::new(&dbus_tool)
        .arg("org.kde.KWin")
        .arg(format!("/Scripting/Script{script_id}"))
        .arg("org.kde.kwin.Script.stop")
        .output();
    let _ = fs::remove_file(&script_path);

    if !journal_output.status.success() {
        return Err(String::from_utf8_lossy(&journal_output.stderr)
            .trim()
            .to_string());
    }

    let mut class_name = String::new();
    let mut caption = String::new();
    for line in String::from_utf8_lossy(&journal_output.stdout).lines() {
        if let Some(value) = line.split("ACTIVE_WINDOW_CLASS:").nth(1) {
            class_name = value.trim().to_string();
        }
        if let Some(value) = line.split("ACTIVE_WINDOW_CAPTION:").nth(1) {
            caption = value.trim().to_string();
        }
        if let Some(value) = line.split("ERROR:").nth(1) {
            return Err(value.trim().to_string());
        }
    }

    if class_name.is_empty() && caption.is_empty() {
        return Err("kwin returned empty active window".to_string());
    }

    Ok(WindowInfo {
        window_title: caption,
        app_name: class_name.clone(),
        process_name: class_name.clone(),
        executable_path: String::new(),
        bundle_id: class_name,
    })
}

fn detect_active_window_with_hyprctl() -> Result<WindowInfo, String> {
    let raw = run_command("hyprctl", &["activewindow", "-j"])?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|err| format!("invalid hyprctl JSON: {err}"))?;

    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let class = value
        .get("class")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let pid = value
        .get("pid")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "missing pid".to_string())?;

    Ok(build_window_info(pid as u32, title, class.clone(), class))
}

fn detect_active_window_with_swaymsg() -> Result<WindowInfo, String> {
    let raw = run_command("swaymsg", &["-t", "get_tree", "-r"])?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|err| format!("invalid swaymsg JSON: {err}"))?;
    let focused = find_focused_node(&value).ok_or_else(|| "no focused node found".to_string())?;

    let title = focused
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let app_name = focused
        .get("app_id")
        .and_then(|v| v.as_str())
        .or_else(|| {
            focused
                .get("window_properties")
                .and_then(|p| p.get("class"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or_default()
        .to_string();
    let pid = focused
        .get("pid")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "missing pid".to_string())?;

    Ok(build_window_info(
        pid as u32,
        title,
        app_name.clone(),
        app_name,
    ))
}

fn find_focused_node<'a>(node: &'a serde_json::Value) -> Option<&'a serde_json::Value> {
    if node
        .get("focused")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        && node.get("pid").is_some()
    {
        return Some(node);
    }

    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(|value| value.as_array()) {
            for child in children {
                if let Some(found) = find_focused_node(child) {
                    return Some(found);
                }
            }
        }
    }

    None
}

fn detect_active_window_x11() -> Result<WindowInfo, String> {
    let mut attempts = 0;
    while attempts < 5 {
        attempts += 1;

        let root = run_command("xprop", &["-root", "_NET_ACTIVE_WINDOW"])?;
        let Some(window_id) = root.split_whitespace().last().map(str::trim) else {
            thread::sleep(Duration::from_millis(150));
            continue;
        };
        if window_id.is_empty() || window_id == "0x0" {
            thread::sleep(Duration::from_millis(150));
            continue;
        }

        let title = run_command("xprop", &["-id", window_id, "_NET_WM_NAME"])
            .or_else(|_| run_command("xprop", &["-id", window_id, "WM_NAME"]))
            .ok()
            .and_then(parse_xprop_quoted_value)
            .unwrap_or_default();

        let class_line = run_command("xprop", &["-id", window_id, "WM_CLASS"])?;
        let app_name = class_line
            .split('"')
            .nth(1)
            .or_else(|| class_line.split('"').nth(3))
            .unwrap_or_default()
            .to_string();

        let pid = run_command("xprop", &["-id", window_id, "_NET_WM_PID"])
            .ok()
            .and_then(|line| line.split_whitespace().last().map(ToString::to_string))
            .and_then(|raw| raw.parse::<u32>().ok())
            .unwrap_or_default();

        if pid > 0 || !app_name.is_empty() || !title.is_empty() {
            return Ok(build_window_info(pid, title, app_name.clone(), app_name));
        }
    }

    Err("xprop failed to resolve active window".to_string())
}

fn detect_active_window_with_xdotool() -> Result<WindowInfo, String> {
    let title = run_command("xdotool", &["getactivewindow", "getwindowname"])?;
    let pid_raw = run_command("xdotool", &["getactivewindow", "getwindowpid"])?;
    let pid = pid_raw
        .trim()
        .parse::<u32>()
        .map_err(|err| format!("invalid pid from xdotool: {err}"))?;

    Ok(build_window_info(
        pid,
        title.trim().to_string(),
        String::new(),
        String::new(),
    ))
}

fn parse_xprop_quoted_value(line: String) -> Option<String> {
    line.split('"').nth(1).map(ToString::to_string)
}

fn build_window_info(
    pid: u32,
    title: String,
    app_name_hint: String,
    bundle_id_hint: String,
) -> WindowInfo {
    let process_name = if pid > 0 {
        process_name_from_pid(pid)
    } else {
        String::new()
    };
    let executable_path = if pid > 0 {
        executable_path_from_pid(pid)
    } else {
        String::new()
    };
    let app_name = if app_name_hint.trim().is_empty() {
        process_name.clone()
    } else {
        app_name_hint
    };

    WindowInfo {
        window_title: title,
        app_name,
        process_name,
        executable_path,
        bundle_id: bundle_id_hint,
    }
}

fn process_name_from_pid(pid: u32) -> String {
    run_command("ps", &["-p", &pid.to_string(), "-o", "comm="])
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

fn executable_path_from_pid(pid: u32) -> String {
    fs::read_link(format!("/proc/{pid}/exe"))
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| err.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!("{program} exited with {}", output.status))
        } else {
            Err(stderr)
        }
    }
}
