use std::sync::{Arc, Mutex};
use std::fs;
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

#[derive(Serialize, Clone)]
struct ConstructorArg {
    name: String,
    arg_type: String,
}

#[derive(Serialize, Clone)]
struct ContractEntry {
    name: String,
    kind: String, // "contract" | "abstract contract" | "interface" | "library"
    constructor_args: Vec<ConstructorArg>,
}

#[derive(Serialize, Clone)]
struct SolFile {
    filename: String,
    path: String,
    contracts: Vec<ContractEntry>,
}

#[tauri::command]
fn scan_contracts(folder: String) -> Result<Vec<SolFile>, String> {
    let mut results: Vec<SolFile> = Vec::new();

    for path in collect_sol_files(std::path::Path::new(&folder)) {
        let source = fs::read_to_string(&path).unwrap_or_default();
        let contracts = parse_contracts(&source);
        results.push(SolFile {
            filename: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            contracts,
        });
    }

    Ok(results)
}

// Parse contract/interface/library/abstract contract declarations line by line.
// Also parses constructor(...) parameter lists within each contract body.
// .lines() strips \r\n and \n automatically — no regex needed.
fn parse_contracts(source: &str) -> Vec<ContractEntry> {
    // Order matters: check "abstract contract" before "contract"
    const KINDS: &[&str] = &["abstract contract", "interface", "library", "contract"];
    let mut out: Vec<ContractEntry> = Vec::new();

    // Track brace depth per contract so we know which contract body we're in.
    // contract_stack: (index into `out`, brace_depth_when_entered)
    let mut contract_stack: Vec<(usize, i32)> = Vec::new();
    let mut brace_depth: i32 = 0;

    for line in source.lines() {
        let t = line.trim();
        // Skip comment lines
        if t.starts_with("//") || t.starts_with("*") || t.starts_with("/*") {
            // Still count braces in block comments is complex; skip for simplicity.
            continue;
        }

        // Check for a new contract/interface/library declaration
        let mut found_decl = false;
        for &kind in KINDS {
            if let Some(rest) = t.strip_prefix(kind) {
                if rest.starts_with(|c: char| c.is_whitespace()) {
                    let name: String = rest
                        .trim_start()
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        let idx = out.len();
                        out.push(ContractEntry {
                            kind: kind.to_string(),
                            name,
                            constructor_args: Vec::new(),
                        });
                        // The opening brace for this contract may be on this line or later.
                        // We'll push to the stack when we see the first '{' after this decl.
                        // Use a sentinel depth of brace_depth+1 (the depth after '{').
                        contract_stack.push((idx, brace_depth + 1));
                        found_decl = true;
                        break;
                    }
                }
            }
        }

        // Check for constructor(...) inside a contract body
        if !found_decl {
            if let Some(current_idx) = contract_stack.last().map(|(i, _)| *i) {
                let trimmed = t;
                if let Some(pos) = trimmed.find("constructor") {
                    let after = trimmed[pos + "constructor".len()..].trim_start();
                    if after.starts_with('(') {
                        // Collect everything up to the matching ')'
                        let paren_content = collect_paren_content(after);
                        let args = parse_param_list(&paren_content);
                        out[current_idx].constructor_args = args;
                    }
                }
            }
        }

        // Count braces to track contract scope
        for ch in t.chars() {
            match ch {
                '{' => {
                    brace_depth += 1;
                }
                '}' => {
                    brace_depth -= 1;
                    // Pop contracts that have ended
                    while let Some(&(_, entered_at)) = contract_stack.last() {
                        if brace_depth < entered_at {
                            contract_stack.pop();
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Given a string starting with '(', return the content between the first '(' and its matching ')'.
fn collect_paren_content(s: &str) -> String {
    let mut depth = 0i32;
    let mut result = String::new();
    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                if depth > 1 { result.push(ch); }
            }
            ')' => {
                depth -= 1;
                if depth == 0 { break; }
                result.push(ch);
            }
            _ => {
                if depth > 0 { result.push(ch); }
            }
        }
    }
    result
}

/// Parse a Solidity parameter list like "uint256 amount, address to, bytes memory data"
/// into a Vec of ConstructorArg { arg_type, name }.
fn parse_param_list(params: &str) -> Vec<ConstructorArg> {
    let params = params.trim();
    if params.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for part in params.split(',') {
        let part = part.trim();
        if part.is_empty() { continue; }
        // Tokens separated by whitespace; last token is name, everything before is type
        // e.g. "uint256 amount", "address payable to", "bytes memory data"
        let tokens: Vec<&str> = part.split_whitespace().collect();
        if tokens.len() >= 2 {
            let name = tokens.last().unwrap_or(&"").to_string();
            // Skip storage location keywords as part of type tokens
            let type_tokens: Vec<&str> = tokens[..tokens.len()-1]
                .iter()
                .filter(|&&t| t != "memory" && t != "storage" && t != "calldata" && t != "payable")
                .copied()
                .collect();
            let arg_type = type_tokens.join(" ");
            if !name.is_empty() && !arg_type.is_empty() {
                out.push(ConstructorArg { name, arg_type });
            }
        } else if tokens.len() == 1 {
            // Unnamed parameter: just a type
            out.push(ConstructorArg { name: String::new(), arg_type: tokens[0].to_string() });
        }
    }
    out
}

fn collect_sol_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return files };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_sol_files(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("sol") {
            files.push(path);
        }
    }
    files
}

// ── Solidity compilation via Node.js + solc npm package ──────────────────
// We can't reliably run Emscripten/WASM in Tauri's WebView, so we shell out
// to Node.js (already on the machine for pnpm) and require() the local solc
// npm package instead.  Input is written to a temp file to avoid any shell-
// escaping problems with large JSON blobs.

#[derive(Serialize, Clone)]
struct CompiledContract {
    abi: String,      // JSON-encoded ABI array
    bytecode: String, // hex string, no 0x prefix
}

#[tauri::command]
async fn compile_contract(
    sources: std::collections::HashMap<String, String>,
    contract_name: String,
) -> Result<CompiledContract, String> {
    // Build Standard JSON input
    let sources_json: serde_json::Map<String, serde_json::Value> = sources
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::json!({ "content": v })))
        .collect();

    let input = serde_json::json!({
        "language": "Solidity",
        "sources": sources_json,
        "settings": {
            "outputSelection": { "*": { "*": ["abi", "evm.bytecode.object"] } }
        }
    });

    let input_str = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    // Write to a temp file so Node doesn't need to receive it on stdin/args
    let tmp = std::env::temp_dir().join("cleric_solc_input.json");
    fs::write(&tmp, &input_str).map_err(|e| e.to_string())?;
    // Use forward slashes so the path is safe inside a JS string literal
    let tmp_js = tmp.to_string_lossy().replace('\\', "/");

    // Require solc from the project's node_modules (CWD = cleric project root
    // when running via `cargo tauri dev`).
    let script = format!(
        r#"const fs=require('fs');
const solc=require('solc');
const out=solc.compile(fs.readFileSync('{tmp}','utf8'));
process.stdout.write(out);
"#,
        tmp = tmp_js
    );

    let node_out = std::process::Command::new("node")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("node not found — is Node.js installed? ({e})"))?;

    let _ = fs::remove_file(&tmp);

    if !node_out.status.success() {
        let stderr = String::from_utf8_lossy(&node_out.stderr);
        return Err(format!("Node error: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&node_out.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Bad solc output: {e}\n{stdout}"))?;

    // Collect compilation errors
    let errors: Vec<String> = result["errors"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|e| e["severity"].as_str() == Some("error"))
        .map(|e| {
            e["formattedMessage"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string()
        })
        .collect();

    if !errors.is_empty() {
        return Err(errors.join("\n").trim().to_string());
    }

    // Find the requested contract in the output
    let contracts = result["contracts"]
        .as_object()
        .ok_or("No contracts in solc output")?;

    for file_contracts in contracts.values() {
        if let Some(contract) = file_contracts.get(&contract_name) {
            let abi = serde_json::to_string(&contract["abi"]).map_err(|e| e.to_string())?;
            let bytecode = contract["evm"]["bytecode"]["object"]
                .as_str()
                .unwrap_or("")
                .to_string();
            if bytecode.is_empty() {
                return Err(format!(
                    "Empty bytecode for '{contract_name}' — is it abstract or an interface?"
                ));
            }
            return Ok(CompiledContract { abi, bytecode });
        }
    }

    Err(format!("Contract '{contract_name}' not found in compilation output"))
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
async fn fund(address: String, amount: u64) -> Result<(), String> {
    let form = reqwest::multipart::Form::new()
        .text("address", address)
        .text("amount", amount.to_string());

    let res = reqwest::Client::new()
        .post("http://localhost:8545/faucet/api")
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        Ok(())
    } else {
        let body: serde_json::Value = res.json().await.unwrap_or_default();
        Err(body["Error"].as_str().unwrap_or("Request failed").to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            child: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            start_druid,
            stop_druid,
            restart_druid,
            get_status,
            fund,
            scan_contracts,
            compile_contract,
            read_file,
            write_file,
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
