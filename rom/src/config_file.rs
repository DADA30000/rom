//! Minimal zero-dependency reader for `$XDG_CONFIG_HOME/rom/config.toml`.
//!
//! Used by [`crate::EngineConfig::default`] and
//! [`crate::RenderConfig::default`] so library embedders (such as `nh`)
//! automatically honor the user's `rom` config file without requiring the `cli`
//! feature or extra dependencies.

use std::{env, fs, io, path::PathBuf, str::FromStr};

use etcetera::{BaseStrategy, choose_base_strategy};
use ratatui_core::style::Color;

use crate::types::{
  Config,
  DisplayFormat,
  HostSort,
  IconMode,
  LegendStyle,
  LogPrefixStyle,
  SummaryStyle,
  Theme,
  TreeDirection,
};

/// Loads `$XDG_CONFIG_HOME/rom/config.toml` onto `config`, ignoring missing or
/// malformed files.
///
/// Set `ROM_NO_CONFIG=1` to skip reading the file.
pub fn apply_user_config(config: &mut Config) {
  if env::var_os("ROM_NO_CONFIG").is_some() {
    return;
  }
  let Some(path) = config_path() else {
    return;
  };
  let text = match fs::read_to_string(&path) {
    Ok(text) => text,
    Err(error) if error.kind() == io::ErrorKind::NotFound => return,
    Err(error) => {
      tracing::warn!(
        path = %path.display(),
        %error,
        "failed to read rom config"
      );
      return;
    },
  };
  apply_toml(&text, config);
}

fn config_path() -> Option<PathBuf> {
  if let Ok(config_home) = env::var("XDG_CONFIG_HOME")
    && !config_home.is_empty()
  {
    return Some(PathBuf::from(config_home).join("rom").join("config.toml"));
  }
  choose_base_strategy()
    .ok()
    .map(|strategy| strategy.config_dir().join("rom").join("config.toml"))
}

enum Section {
  Root,
  Theme,
  Other,
}

fn apply_toml(text: &str, config: &mut Config) {
  let mut section = Section::Root;

  for raw_line in text.lines() {
    let line = strip_comment(raw_line).trim();
    if line.is_empty() {
      continue;
    }
    if let Some(header) =
      line.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
    {
      section = match header.trim() {
        "theme" => Section::Theme,
        _ => Section::Other,
      };
      continue;
    }
    let Some((raw_key, raw_value)) = line.split_once('=') else {
      continue;
    };
    let key = unquote(raw_key.trim());
    let value = raw_value.trim();

    match section {
      Section::Root => apply_root_key(key, value, config),
      Section::Theme => apply_theme_key(key, value, &mut config.render.theme),
      Section::Other => {},
    }
  }
}

fn strip_comment(line: &str) -> &str {
  let mut in_double = false;
  let mut in_single = false;
  let mut escaped = false;

  for (index, ch) in line.char_indices() {
    if escaped {
      escaped = false;
      continue;
    }
    if in_double {
      if ch == '\\' {
        escaped = true;
      } else if ch == '"' {
        in_double = false;
      }
      continue;
    }
    if in_single {
      if ch == '\'' {
        in_single = false;
      }
      continue;
    }
    match ch {
      '"' => in_double = true,
      '\'' => in_single = true,
      '#' => return &line[..index],
      _ => {},
    }
  }
  line
}

fn unquote(value: &str) -> &str {
  parse_string(value).unwrap_or(value)
}

fn parse_string(value: &str) -> Option<&str> {
  if let Some(inner) = value.strip_prefix('"') {
    return inner.strip_suffix('"');
  }
  value.strip_prefix('\'')?.strip_suffix('\'')
}

fn parse_bool(value: &str) -> Option<bool> {
  match value {
    "true" => Some(true),
    "false" => Some(false),
    _ => None,
  }
}

fn parse_usize(value: &str) -> Option<usize> {
  let clean = value.replace('_', "");
  clean.parse::<usize>().ok()
}

fn apply_root_key(key: &str, value: &str, config: &mut Config) {
  match key {
    "format" => {
      if let Some(word) = parse_string(value) {
        match word {
          "tree" => config.render.format = DisplayFormat::Tree,
          "plain" => config.render.format = DisplayFormat::Plain,
          "dashboard" => config.render.format = DisplayFormat::Dashboard,
          _ => {},
        }
      }
    },
    "tree-direction" => {
      if let Some(word) = parse_string(value) {
        match word {
          "down" => config.render.tree_direction = TreeDirection::Down,
          "up" => config.render.tree_direction = TreeDirection::Up,
          _ => {},
        }
      }
    },
    "legend" => {
      if let Some(word) = parse_string(value) {
        match word {
          "compact" => config.render.legend_style = LegendStyle::Compact,
          "table" => config.render.legend_style = LegendStyle::Table,
          "verbose" => config.render.legend_style = LegendStyle::Verbose,
          "nom" => config.render.legend_style = LegendStyle::Nom,
          _ => {},
        }
      }
    },
    "summary" => {
      if let Some(word) = parse_string(value) {
        match word {
          "concise" => config.render.summary_style = SummaryStyle::Concise,
          "table" => config.render.summary_style = SummaryStyle::Table,
          "full" => config.render.summary_style = SummaryStyle::Full,
          "nom" => config.render.summary_style = SummaryStyle::Nom,
          _ => {},
        }
      }
    },
    "icons" => {
      if let Some(word) = parse_string(value) {
        match word {
          "auto" => config.render.icons = IconMode::Auto,
          "unicode" => config.render.icons = IconMode::Unicode,
          "nerd" => config.render.icons = IconMode::Nerd,
          _ => {},
        }
      }
    },
    "log-prefix" => {
      if let Some(word) = parse_string(value) {
        match word {
          "short" => config.engine.log_prefix_style = LogPrefixStyle::Short,
          "full" => config.engine.log_prefix_style = LogPrefixStyle::Full,
          "none" => config.engine.log_prefix_style = LogPrefixStyle::None,
          _ => {},
        }
      }
    },
    "log-lines" => {
      if let Some(limit) = parse_usize(value) {
        config.engine.log_line_limit = Some(limit);
      }
    },
    "host-sort" => {
      if let Some(word) = parse_string(value) {
        match word {
          "none" => config.render.host_sort = HostSort::None,
          "download-size" => config.render.host_sort = HostSort::DownloadSize,
          "download-count" => config.render.host_sort = HostSort::DownloadCount,
          "builds" => config.render.host_sort = HostSort::Builds,
          _ => {},
        }
      }
    },
    "max-hosts" => {
      if let Some(cap) = parse_usize(value) {
        config.render.max_hosts = Some(cap);
      }
    },
    "curl-progress" => {
      if let Some(enabled) = parse_bool(value) {
        config.engine.curl_progress = enabled;
      }
    },
    "progress-bars" => {
      if let Some(enabled) = parse_bool(value) {
        config.render.progress_bars = enabled;
      }
    },
    "show-percentages" => {
      if let Some(enabled) = parse_bool(value) {
        config.render.show_percentages = enabled;
      }
    },
    "decimal-bytes" => {
      if let Some(enabled) = parse_bool(value) {
        config.render.decimal_bytes = enabled;
      }
    },
    _ => {},
  }
}

fn apply_theme_key(key: &str, value: &str, theme: &mut Theme) {
  let Some(raw) = parse_string(value) else {
    return;
  };
  let Ok(color) = Color::from_str(raw) else {
    return;
  };
  match key {
    "connector" => theme.connector = color,
    "text" => theme.text = color,
    "muted" => theme.muted = color,
    "timer" => theme.timer = color,
    "planned" => theme.planned = color,
    "running" => theme.running = color,
    "completed" => theme.completed = color,
    "failed" => theme.failed = color,
    "log-prefix" => theme.log_prefix = color,
    "host" => theme.host = color,
    "download" => theme.download = color,
    "upload" => theme.upload = color,
    "transfer-bytes" => theme.transfer_bytes = color,
    "progress-track" => theme.progress_track = color,
    "cpu" => theme.cpu = color,
    "cpu-serial" => theme.cpu_serial = color,
    "cpu-idle" => theme.cpu_idle = color,
    "cpu-heavy" => theme.cpu_heavy = color,
    _ => {},
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_full_config_and_theme_without_toml_crate() {
    let mut config = Config::plain_default();
    apply_toml(
      "\
# comment
tree-direction = \"up\" # inline comment
legend = \"nom\"
summary = \"nom\"
icons = \"unicode\"
curl-progress = true
host-sort = \"download-size\"
max-hosts = 5
progress-bars = false
show-percentages = false
decimal-bytes = true

[theme]
connector = \"reset\"
log-prefix = \"blue\"
download = \"yellow\"
transfer-bytes = \"green\"
host = \"#ff8800\" # hex color with hash inside quotes
",
      &mut config,
    );
    assert_eq!(config.render.tree_direction, TreeDirection::Up);
    assert_eq!(config.render.legend_style, LegendStyle::Nom);
    assert_eq!(config.render.summary_style, SummaryStyle::Nom);
    assert_eq!(config.render.icons, IconMode::Unicode);
    assert!(config.engine.curl_progress);
    assert_eq!(config.render.host_sort, HostSort::DownloadSize);
    assert_eq!(config.render.max_hosts, Some(5));
    assert!(!config.render.progress_bars);
    assert!(!config.render.show_percentages);
    assert!(config.render.decimal_bytes);
    assert_eq!(config.render.theme.connector, Color::Reset);
    assert_eq!(config.render.theme.log_prefix, Color::Blue);
    assert_eq!(config.render.theme.download, Color::Yellow);
    assert_eq!(config.render.theme.transfer_bytes, Color::Green);
    assert_eq!(config.render.theme.host, Color::Rgb(0xFF, 0x88, 0x00));
  }
}
