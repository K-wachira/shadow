mod tui;

use db::Database;
use llm::LlmClient;
use ratatui::Terminal;
use ratatui::TerminalOptions;
use ratatui::Viewport;
use ratatui::backend::CrosstermBackend;
use shadow_core::locus::Locus;
use shadow_core::setup;
use shadow_core::*;
use std::env;
use std::io;
use std::sync::Arc;
use tracing_subscriber;
use tui::run;

#[tokio::main]
#[hotpath::main]
async fn main() -> color_eyre::Result<()> {
    cli_main().await
}

async fn cli_main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let (config, paths) = setup::run_setup()?;

    // `shadow identity ...` — a headless surface for scripting and testing
    // (no TUI, no Ollama). Runs before the terminal is taken over.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("identity") {
        let sub = args.get(2).map(String::as_str).unwrap_or("show");
        return run_identity_cli(sub, &args[3.min(args.len())..], &paths, &config.identity);
    }

    let log_file = std::fs::OpenOptions::new().append(true).open(&paths.log)?;

    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let db_conn = Arc::new(Database::init(&paths.db)?);

    let llm_client = Arc::new(LlmClient::init(&config)?);

    let identity = Arc::new(shadow_core::identity::unlock_or_init(
        &paths,
        &config.identity,
    )?);

    let mut locus = Locus::new(db_conn, llm_client, config, paths, identity)?;

    crossterm::terminal::enable_raw_mode()?;
    let terminal_height = crossterm::terminal::size()?.1;
    let viewport = configured_viewport(terminal_height);

    let terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions { viewport },
    )?;

    let result = run(terminal, &mut locus).await;
    crossterm::terminal::disable_raw_mode()?;
    result?;

    println!("");
    Ok(())
}

/// Headless identity commands: `shadow identity [show|sign <msg>|verify <sig_hex> <msg>]`.
fn run_identity_cli(
    sub: &str, rest: &[String], paths: &setup::ShadowPaths,
    id_cfg: &shadow_core::config::IdentityConfig,
) -> color_eyre::Result<()> {
    use color_eyre::eyre::eyre;

    // surface the dev-passphrase warning on stderr (the TUI logs to a file).
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::WARN)
        .try_init();

    // `restore` runs before unlock — it *creates* the identity from a backup.
    if sub == "restore" {
        let inp = rest
            .first()
            .ok_or_else(|| eyre!("usage: shadow identity restore <in-file>"))?;
        let blob = std::fs::read(inp)?;
        let id =
            shadow_core::identity::restore_backup(paths, &blob, &backup_passphrase()?, id_cfg)?;
        println!("restored shadow id: {}", to_hex(&id.to_bytes()));
        return Ok(());
    }

    let identity = shadow_core::identity::unlock_or_init(paths, id_cfg)?;

    match sub {
        "show" => {
            println!("shadow id: {}", to_hex(&identity.shadow_id.to_bytes()));
            let at_rest = match std::fs::read(&paths.mind) {
                Ok(bytes)
                    if shadow_identity::Vault::new(&identity.dek)
                        .open(&bytes)
                        .is_ok() =>
                {
                    "encrypted (sealed)"
                }
                Ok(_) => "PLAINTEXT — not sealed yet; seals on next save",
                Err(_) => "absent (no mind file yet)",
            };
            println!("mind at rest: {at_rest}");
        }
        "sign" => {
            if rest.is_empty() {
                return Err(eyre!("usage: shadow identity sign <message>"));
            }
            let msg = rest.join(" ");
            let sig = identity.keypair.sign(msg.as_bytes());
            println!("{}", to_hex(&sig.to_bytes()));
        }
        "verify" => {
            if rest.len() < 2 {
                return Err(eyre!("usage: shadow identity verify <sig_hex> <message>"));
            }
            let bytes = from_hex(&rest[0]).ok_or_else(|| eyre!("signature is not valid hex"))?;
            let arr: [u8; 64] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| eyre!("signature must be 64 bytes (128 hex chars)"))?;
            let sig = shadow_identity::Signature::from_bytes(&arr);
            let msg = rest[1..].join(" ");
            match identity.shadow_id.verify(msg.as_bytes(), &sig) {
                Ok(()) => println!("VALID"),
                Err(_) => println!("INVALID"),
            }
        }
        "backup" => {
            let out = rest
                .first()
                .ok_or_else(|| eyre!("usage: shadow identity backup <out-file>"))?;
            let blob = shadow_core::identity::export_backup(&identity, &backup_passphrase()?)?;
            std::fs::write(out, &blob)?;
            println!("backup written: {out} ({} bytes)", blob.len());
        }
        other => {
            return Err(eyre!(
                "unknown subcommand `{other}` (use: show | sign | verify | backup | restore)"
            ));
        }
    }
    Ok(())
}

/// The recovery passphrase for offline backups — kept distinct from the device
/// keystore passphrase so a backup survives losing the device.
fn backup_passphrase() -> color_eyre::Result<Vec<u8>> {
    match std::env::var("SHADOW_BACKUP_PASSPHRASE") {
        Ok(p) if !p.is_empty() => Ok(p.into_bytes()),
        _ => Err(color_eyre::eyre::eyre!(
            "set SHADOW_BACKUP_PASSPHRASE (the recovery passphrase for the backup file)"
        )),
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn configured_viewport(terminal_height: u16) -> Viewport {
    match env::var("SHADOW_TUI_VIEWPORT").ok().as_deref() {
        Some("fullscreen") => Viewport::Fullscreen,
        _ => Viewport::Inline(default_inline_height(terminal_height)),
    }
}

fn default_inline_height(terminal_height: u16) -> u16 {
    let reserved_rows = if terminal_height >= 20 {
        4
    } else if terminal_height >= 12 {
        2
    } else {
        1
    };

    terminal_height.saturating_sub(reserved_rows).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_inline_height_leaves_rows_above_viewport() {
        assert_eq!(default_inline_height(24), 20);
        assert_eq!(default_inline_height(16), 14);
        assert_eq!(default_inline_height(8), 7);
    }
}
