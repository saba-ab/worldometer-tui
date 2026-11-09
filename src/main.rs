//! worldo-tui entry point: CLI parsing, terminal setup/teardown and the event loop.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use chrono::{Local, Offset};
use clap::Parser;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use worldo_tui::app::App;
use worldo_tui::config::{self, Config};
use worldo_tui::counters::{self, Counter};
use worldo_tui::export;
use worldo_tui::msg::{Msg, Request};
use worldo_tui::net::cache::Cache;
use worldo_tui::net::worker::{self, Loader};
use worldo_tui::tables::TableKind;
use worldo_tui::ui::{self, Theme};

const TICK: Duration = Duration::from_millis(100);

/// Browse worldometers.info statistics in the terminal.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Write a table to stdout instead of starting the UI.
    /// FORMAT: csv. TABLE: countries | gdp | co2 | area.
    #[arg(long, num_args = 2, value_names = ["FORMAT", "TABLE"])]
    export: Option<Vec<String>>,

    /// Never touch the network; use cached pages only.
    #[arg(long)]
    offline: bool,

    /// Config file (default: config.toml in the platform config directory).
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli.config.clone().or_else(config::default_config_path);
    let config = match &config_path {
        Some(path) => Config::load(path)?,
        None => Config::default(),
    };
    let cache_dir = Cache::default_dir().context("cannot determine a cache directory")?;
    let cache = Cache::new(cache_dir);

    if let Some(args) = &cli.export {
        return run_export(args, Loader::new(cache, config.ttl(), cli.offline));
    }

    let estimated = load_estimated_counters();
    let tz = Local::now().offset().fix();
    let mut app = App::new(
        config.default_tab,
        estimated,
        config.ttl(),
        tz,
        Theme::from_name(config.theme),
    );
    let (requests, messages) = worker::spawn(cache, config.ttl(), cli.offline);

    // `ratatui::init` installs a panic hook that restores the terminal.
    let terminal = ratatui::init();
    let result = run(terminal, &mut app, &requests, &messages);
    ratatui::restore();
    result
}

fn run_export(args: &[String], loader: Loader) -> Result<()> {
    let [format, table] = args else {
        bail!("usage: --export csv <table>");
    };
    if !format.eq_ignore_ascii_case("csv") {
        bail!("unsupported export format {format:?} (only \"csv\")");
    }
    let names: Vec<&str> = TableKind::ALL.iter().map(|k| k.cli_name()).collect();
    let kind = TableKind::from_cli_name(table).with_context(|| {
        format!(
            "unknown table {table:?}; expected one of {}",
            names.join(", ")
        )
    })?;
    let csv = export::export_csv(&loader, kind).map_err(anyhow::Error::msg)?;
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(csv.as_bytes())
        .and_then(|()| stdout.flush())
    {
        // `worldo-tui --export csv countries | head` closes the pipe early; that's fine.
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.context("writing CSV to stdout"),
    }
}

/// Option B counters: a user override in the config dir, else the bundled file.
fn load_estimated_counters() -> Result<Vec<Counter>, String> {
    let override_src = config::counters_override_path().and_then(|p| fs::read_to_string(p).ok());
    let src = override_src
        .as_deref()
        .unwrap_or(counters::DEFAULT_COUNTERS_TOML);
    counters::load_estimated(src).map_err(|e| e.to_string())
}

fn run(
    mut terminal: DefaultTerminal,
    app: &mut App,
    requests: &Sender<Request>,
    messages: &Receiver<Msg>,
) -> Result<()> {
    loop {
        for request in app.take_requests() {
            requests
                .send(request)
                .context("fetch worker stopped unexpectedly")?;
        }

        terminal.draw(|frame| ui::draw(frame, app, SystemTime::now()))?;

        if event::poll(TICK)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }
        while let Ok(msg) = messages.try_recv() {
            app.on_msg(msg);
        }
        if app.should_quit {
            return Ok(());
        }
    }
}
