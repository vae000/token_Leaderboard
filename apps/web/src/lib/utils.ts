export function cn(...inputs: Array<string | false | null | undefined>) {
  return inputs.filter(Boolean).join(" ");
}

const TOOL_DISPLAY_NAMES: Record<string, string> = {
  deepseek_tui: "DeepSeek-TUI",
  codex: "Codex",
};

export function formatToolName(raw: string): string {
  return TOOL_DISPLAY_NAMES[raw] ?? raw;
}
