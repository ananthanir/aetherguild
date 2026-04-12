use std::sync::{Arc, Mutex};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_shell::{ShellExt, process::CommandChild, process::CommandEvent};

struct AppState {
    child: Arc<Mutex<Option<CommandChild>>>,
}

#[derive(Clone, Serialize)]
struct LogPayload {
    line: String,
    stream: String,
}

#[derive(Clone, Serialize)]
struct StatusPayload {
    status: String,
}

#[tauri::command]
async fn start_druid(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Check if already running
    {
        let child = state.child.lock().map_err(|e| e.to_string())?;
        if child.is_some() {
            return Err("Druid is already running".into());
        }
    }

    eprintln!("[cleric] Attempting to spawn druid sidecar...");

    let cmd = app
        .shell()
        .sidecar("druid")
        .map_err(|e| {
            let msg = format!("Failed to create sidecar command: {}", e);
            eprintln!("[cleric] {}", msg);
            msg
        })?;

    eprintln!("[cleric] Sidecar command created, spawning...");

    let (mut rx, child) = cmd
        .spawn()
        .map_err(|e| {
            let msg = format!("Failed to spawn druid: {}", e);
            eprintln!("[cleric] {}", msg);
            msg
        })?;

    eprintln!("[cleric] Druid spawned successfully!");

    // Store the child process
    {
        let mut guard = state.child.lock().map_err(|e| e.to_string())?;
        *guard = Some(child);
    }

    // Emit running status
    let _ = app.emit("druid-status", StatusPayload { status: "running".into() });

    // Spawn async task to read stdout/stderr and forward as events
    let app_handle = app.clone();
    let child_mutex = state.child.clone();

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let line_str = String::from_utf8_lossy(&line).to_string();
                    eprintln!("[cleric:stdout] {}", line_str);
                    let _ = app_handle.emit("druid-log", LogPayload {
                        line: line_str,
                        stream: "stdout".into(),
                    });
                }
                CommandEvent::Stderr(line) => {
                    let line_str = String::from_utf8_lossy(&line).to_string();
                    eprintln!("[cleric:stderr] {}", line_str);
                    let _ = app_handle.emit("druid-log", LogPayload {
                        line: line_str,
                        stream: "stderr".into(),
                    });
                }
                CommandEvent::Terminated(payload) => {
                    let msg = match payload.code {
                        Some(code) => format!("Process exited with code {}", code),
                        None => "Process terminated".into(),
                    };
                    eprintln!("[cleric] {}", msg);
                    let _ = app_handle.emit("druid-log", LogPayload {
                        line: msg,
                        stream: "stderr".into(),
                    });
                    let _ = app_handle.emit("druid-status", StatusPayload {
                        status: "stopped".into(),
                    });
                    // Clear child from state
                    if let Ok(mut guard) = child_mutex.lock() {
                        *guard = None;
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_druid(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.child.lock().map_err(|e| e.to_string())?;
    match guard.take() {
        Some(child) => {
            child.kill().map_err(|e| format!("Failed to kill druid: {}", e))?;
            Ok(())
        }
        None => Err("Druid is not running".into()),
    }
}

#[tauri::command]
async fn restart_druid(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Stop if running (ignore error if not running)
    {
        let mut guard = state.child.lock().map_err(|e| e.to_string())?;
        if let Some(child) = guard.take() {
            let _ = child.kill();
        }
    }

    // Brief pause to let the process fully terminate
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Start fresh
    start_druid(app, state).await
}

#[tauri::command]
fn get_status(state: State<'_, AppState>) -> String {
    match state.child.lock() {
        Ok(guard) => {
            if guard.is_some() { "running".into() } else { "stopped".into() }
        }
        Err(_) => "stopped".into(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            child: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            start_druid,
            stop_druid,
            restart_druid,
            get_status,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                // Kill druid when the app exits
                let child_arc = app.state::<AppState>().child.clone();
                let mut guard = match child_arc.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                if let Some(child) = guard.take() {
                    eprintln!("[cleric] Killing druid on app exit...");
                    let _ = child.kill();
                }
            }
        });
}
