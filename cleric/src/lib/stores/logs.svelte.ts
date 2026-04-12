import { listen } from "@tauri-apps/api/event";

export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

const MAX_LOGS = 10000;

function detectLevel(line: string): string {
  if (line.includes("ERROR") || line.includes("CRIT")) return "ERROR";
  if (line.includes("WARN")) return "WARN";
  if (line.includes("DEBUG") || line.includes("TRACE")) return "DEBUG";
  return "INFO";
}

function nowTimestamp(): string {
  const d = new Date();
  return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}:${d.getSeconds().toString().padStart(2, "0")}.${d.getMilliseconds().toString().padStart(3, "0")}`;
}

let entries = $state<LogEntry[]>([]);

export const logStore = {
  get entries() { return entries; },
  clear() { entries = []; },
};

let listening = false;

export function initLogListener() {
  if (listening) return;
  listening = true;

  listen<{ line: string; stream: string }>("druid-log", (event) => {
    const line = event.payload.line.trim();
    if (!line) return;

    const entry: LogEntry = {
      timestamp: nowTimestamp(),
      level: detectLevel(line),
      message: line,
    };

    entries = [...entries, entry].slice(-MAX_LOGS);
  }).catch((e) => {
    console.warn("Failed to listen for druid-log:", e);
    listening = false;
  });
}
