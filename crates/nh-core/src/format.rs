use crate::ui::*;
use std::io::{BufRead, BufReader};
use subprocess::Exec;
use yansi::Paint;

pub enum LogLine {
  HomebrewUsing(String),
  HomebrewComplete(usize),
  HomeManagerActivating(String),
  DarwinInfo(String),
  DarwinSuccess(String),
  Other(String),
}

pub fn process_line(line: &str) -> LogLine {
  let line = line.trim();
  if let Some(dep) = line.strip_prefix("Using ") {
    return LogLine::HomebrewUsing(dep.to_string());
  }
  if let Some(dep) = line.strip_prefix("Installing ") {
    return LogLine::HomebrewUsing(dep.to_string());
  }
  if let Some(caps) = line.strip_prefix("`brew bundle` complete! ") {
    if let Some(count_str) = caps.split_whitespace().next() {
      if let Ok(count) = count_str.parse::<usize>() {
        return LogLine::HomebrewComplete(count);
      }
    }
  }
  if let Some(module) = line.strip_prefix("Activating ") {
    return LogLine::HomeManagerActivating(module.to_string());
  }
  if let Some(msg) = line.strip_prefix("✓ ") {
    return LogLine::DarwinSuccess(msg.to_string());
  }
  if let Some(info) = line.strip_prefix("ℹ️ ") {
    return LogLine::DarwinInfo(info.to_string());
  }
  if line.contains("━━━") || line.contains("━━━━") {
    let content = line.trim_matches('━').trim();
    if !content.is_empty() {
      return LogLine::DarwinInfo(content.to_string());
    }
  }

  LogLine::Other(line.to_string())
}

pub struct ActivationState {
  brew_count: usize,
  hm_count: usize,
  last_info: Option<String>,
}

impl Default for ActivationState {
  fn default() -> Self {
    Self {
      brew_count: 0,
      hm_count: 0,
      last_info: None,
    }
  }
}

pub fn run_pretty(exec: Exec) -> color_eyre::Result<()> {
  let mut popen = exec.start()?;
  let stdout = popen.stdout.take().unwrap();
  let reader = BufReader::new(stdout);
  let mut state = ActivationState::default();

  for line_result in reader.lines() {
    let line = line_result?;
    if line.trim().is_empty() {
      continue;
    }

    match process_line(&line) {
      LogLine::HomebrewUsing(_) => {
        state.brew_count += 1;
        print!(
          "\r  {} Homebrew: {} dependencies processed",
          Paint::new(ICON_INFO).fg(BLUE),
          state.brew_count
        );
        use std::io::Write;
        std::io::stdout().flush()?;
      },
      LogLine::HomebrewComplete(count) => {
        println!(
          "\r  {} Homebrew: {} dependencies now installed",
          Paint::new(ICON_SUCCESS).fg(GREEN),
          count
        );
      },
      LogLine::HomeManagerActivating(_) => {
        state.hm_count += 1;
        print!(
          "\r  {} Home-Manager: {} modules activated",
          Paint::new(ICON_INFO).fg(BLUE),
          state.hm_count
        );
        use std::io::Write;
        std::io::stdout().flush()?;
      },
      LogLine::DarwinInfo(info) => {
        if state.last_info.as_ref() != Some(&info) {
          println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), info);
          state.last_info = Some(info);
        }
      },
      LogLine::DarwinSuccess(msg) => {
        println!("  {} {}", Paint::new(ICON_SUCCESS).fg(GREEN), msg);
      },
      LogLine::Other(other) => {
        if other.contains("Starting Home Manager activation") {
          println!();
        } else if other.contains("Error:") || other.contains("failed") {
          println!(
            "  {} {}",
            Paint::new(ICON_WARNING).fg(RED),
            Paint::new(other).fg(RED)
          );
        } else {
          // debug!("Ignored line: {}", other);
        }
      },
    }
  }

  let status = popen.wait()?;
  if !status.success() {
    return Err(color_eyre::eyre::eyre!(
      "Activation failed with status {:?}",
      status
    ));
  }

  Ok(())
}
