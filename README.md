# Shadow

A local-first personal AI assistant and life logging system. Shadow doesn't just store what you tell it — it surfaces truths you didn't explicitly state.

## Philosophy

Most personal data tools are databases with a search box. Shadow is different. The goal is an intelligence layer: a system that observes your logs over time, rewrites its own understanding of you, and eventually says something true about you that you never told it.

Privacy is non-negotiable. Everything runs locally. No cloud inference. No external data exposure.

## Architecture

```
Logs (iPhone Shortcut → JSON → iCloud Drive)
         ↓
   SQLite (append-only raw log store)
         ↓
   Ollama (local inference — gemma3:12b)
         ↓
   shadow.mind (living document, periodically rewritten by Ollama)
```

**SQLite** is the raw truth. Append-only. Never mutated.  
**Ollama** is the author, not a query engine.  
**`shadow.mind`** is a structured file (JSON/TOML) holding versioned beliefs, confidence gradients, and a layered model of you — not a database table.

## Features

- **Log ingestion** — iPhone Shortcut captures content, energy, mood, location, weather, time, and device. Ingested via CLI from iCloud Drive.
- **Local inference** — All AI runs on-device via Ollama. No data leaves your machine.
- **Streaming responses** — Markdown-rendered output, streamed in real time.
- **TUI interface** — Terminal UI built with `ratatui` + `tui-textarea`.
- **`shadow.mind` rewrite cycle** — Ollama periodically authors a structured self-model from your logs. It tracks beliefs with confidence scores, behavioral patterns, and the negative space (what's conspicuously absent).
- **CLI** — Full-featured command-line interface for ingestion, querying, and inspection.

## Stack

| Layer | Technology |
|---|---|
| Language | Rust |
| Database | SQLite (`rusqlite`) |
| Inference | Ollama (`gemma3:12b`) |
| CLI | `clap` |
| TUI | `ratatui` + `tui-textarea` |
| Async | `tokio` |
| Errors | `color_eyre` |
| Logging | `tracing` |
| Markdown | `pulldown-cmark` / `tui-markdown` |

## Project Structure

```
src/
├── main.rs         # Entry point
├── commands.rs     # CLI command definitions
├── db.rs           # SQLite interactions
├── models.rs       # Data models
├── handlers.rs     # Command handlers
├── ask.rs          # Query interface
├── ingest.rs       # Log ingestion pipeline
├── ollama.rs       # Ollama client + streaming
└── tui.rs          # Terminal UI
```

## Getting Started

### Prerequisites

- Rust (stable)
- [Ollama](https://ollama.ai) running locally
- `gemma3:12b` pulled: `ollama pull gemma3:12b`

### Build

```bash
git clone https://github.com/yourname/shadow
cd shadow
cargo build --release
```

### Ingest logs

```bash
shadow ingest --path ~/path/to/logs.json
```

### Ask Shadow something

```bash
shadow ask "what has my energy been like this week?"
```

### Launch TUI

```bash
shadow tui
```

## Log Format

Logs are captured via iPhone Shortcut and exported as JSON. Each entry includes:

```json
{
  "content": "...",
  "energy": "high|medium|low",
  "mood": "...",
  "location": "...",
  "weather": "...",
  "timestamp": "...",
  "device": "iPhone"
}
```

Log quality matters more than log quantity. Each entry should be evaluable by asking: *could Ollama infer something real from this?*

## `shadow.mind`

The living document at the core of Shadow. Ollama authors and rewrites it periodically — it is not generated on-demand. It has seven properties that distinguish it from a database:

1. **Versioned beliefs** — prior model is preserved when overwritten
2. **Confidence gradients** — every belief carries a confidence score
3. **Layered structure** — surface behaviours, patterns, mental models, and values update at different rates
4. **Relational nodes** — beliefs connect to each other, not just to log entries
5. **Negative space tracking** — what's notably absent is as meaningful as what's present
6. **Temporal decay with selective preservation** — recent signal weighs more, but high-confidence beliefs persist
7. **Meta-awareness** — Shadow tracks its own gaps and uncertainties

User corrections carry higher confidence than inferred beliefs.

## Roadmap

- [x] V1 — Models, DB, ingestion, CLI
- [x] V2 — Ollama integration, streaming, markdown rendering
- [ ] V2.5 — TUI completion, slash commands (`/ingest`, `/clear`, `/mood`)
- [ ] V3 — Embeddings, entity extraction, knowledge graph (`sqlite-vec`)
- [ ] V4 — Full chat interface, `/loop` command for recurring prompts

## Hardware

Developed on macOS, M3 Max, 36GB unified memory. Inference is fast enough to feel live.

## License

MIT