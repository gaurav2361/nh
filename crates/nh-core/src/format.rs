use crate::ui::*;
use std::io::{BufRead, BufReader};
use subprocess::Exec;
use yansi::Paint;

pub enum LogLine {
  HomebrewUsing(String),
  HomebrewInstalling(String),
  HomebrewUpgrading(String),
  HomebrewComplete(usize),
  HomeManagerActivating(String),
  DarwinInfo(String),
  DarwinSuccess(String),
  SectionHeader(String),
  FlakeInputUpdating(String),
  Warning(String),
  Other(String),
}

pub fn process_line(line: &str) -> LogLine {
  let line = line.trim();
  if let Some(dep) = line.strip_prefix("Using ") {
    return LogLine::HomebrewUsing(dep.to_string());
  }
  if let Some(dep) = line.strip_prefix("Installing ") {
    return LogLine::HomebrewInstalling(dep.to_string());
  }
  if let Some(dep) = line.strip_prefix("Upgrading ") {
    return LogLine::HomebrewUpgrading(dep.to_string());
  }
  if let Some(caps) = line.strip_prefix("`brew bundle` complete! ") {
    if let Some(count_str) = caps.split_whitespace().next() {
      if let Ok(count) = count_str.parse::<usize>() {
        return LogLine::HomebrewComplete(count);
      }
    }
  }
  if let Some(caps) = line.strip_prefix("Homebrew Bundle complete! ") {
    if let Some(count_str) = caps.split_whitespace().next() {
      if let Ok(count) = count_str.parse::<usize>() {
        return LogLine::HomebrewComplete(count);
      }
    }
  }
  if let Some(input) = line.strip_prefix("updating input '") {
    let input = input.trim_end_matches('\'');
    return LogLine::FlakeInputUpdating(input.to_string());
  }
  if let Some(module) = line.strip_prefix("Activating ") {
    return LogLine::HomeManagerActivating(module.to_string());
  }
  if let Some(msg) = line.strip_prefix("✓ ").or_else(|| line.strip_prefix("✔ ")) {
    if msg.contains("Error:") || msg.contains("failed") {
      return LogLine::Warning(msg.to_string());
    }
    return LogLine::DarwinSuccess(msg.to_string());
  }
  if let Some(section) = line.strip_prefix("➜ ") {
    return LogLine::SectionHeader(section.to_string());
  }
  if let Some(info) = line.strip_prefix("ℹ️ ") {
    return LogLine::DarwinInfo(info.to_string());
  }
  if let Some(warn) = line.strip_prefix("Warning: ") {
    return LogLine::Warning(warn.to_string());
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
  brew_using: usize,
  brew_installing: usize,
  brew_upgrading: usize,
  brew_missing: usize,
  hm_count: usize,
  darwin_success: usize,
  flake_updates: usize,
  last_info: Option<String>,
}

impl Default for ActivationState {
  fn default() -> Self {
    Self {
      brew_using: 0,
      brew_installing: 0,
      brew_upgrading: 0,
      brew_missing: 0,
      hm_count: 0,
      darwin_success: 0,
      flake_updates: 0,
      last_info: None,
    }
  }
}

fn print_brew_status(state: &ActivationState) {
  print!(
    "\r\x1b[2K  {} Homebrew: {} installed, {} upgrading, {} working fine",
    Paint::new(crate::ui::ICON_INFO).fg(crate::ui::BLUE),
    state.brew_installing,
    state.brew_upgrading,
    state.brew_using
  );
  if state.brew_missing > 0 {
    print!(" ({} missing)", state.brew_missing);
  }
  use std::io::Write;
  let _ = std::io::stdout().flush();
}

pub fn run_pretty(exec: Exec) -> color_eyre::Result<()> {
  let mut popen = exec.start()?;
  let stdout = popen.stdout.take().ok_or_else(|| color_eyre::eyre::eyre!("Failed to capture stdout"))?;
  let reader = BufReader::new(stdout);
  let mut state = ActivationState::default();

  for line_result in reader.lines() {
    let line = line_result?;
    if line.trim().is_empty() {
      continue;
    }

    match process_line(&line) {
      LogLine::HomebrewUsing(_) => {
        state.brew_using += 1;
        print_brew_status(&state);
      },
      LogLine::HomebrewInstalling(_) => {
        state.brew_installing += 1;
        print_brew_status(&state);
      },
      LogLine::HomebrewUpgrading(_) => {
        state.brew_upgrading += 1;
        print_brew_status(&state);
      },
      LogLine::HomebrewComplete(count) => {
        println!(
          "\r\x1b[2K  {} Homebrew: {} dependencies processed ({} installed, {} upgraded, {} working fine{})",
          Paint::new(ICON_SUCCESS).fg(GREEN),
          count,
          state.brew_installing,
          state.brew_upgrading,
          state.brew_using,
          if state.brew_missing > 0 {
            format!(", {} missing", state.brew_missing)
          } else {
            "".to_string()
          }
        );
      },
      LogLine::HomeManagerActivating(_) => {
        state.hm_count += 1;
        print!(
          "\r\x1b[2K  {} Home-Manager: {} modules activated",
          Paint::new(ICON_INFO).fg(BLUE),
          state.hm_count
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
      },
      LogLine::FlakeInputUpdating(_) => {
        state.flake_updates += 1;
        print!(
          "\r\x1b[2K  {} Flake: {} inputs updated",
          Paint::new(ICON_INFO).fg(BLUE),
          state.flake_updates
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
      },
      LogLine::DarwinInfo(info) => {
        if state.last_info.as_ref() != Some(&info) {
          println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), info);
          state.last_info = Some(info);
        }
      },
      LogLine::DarwinSuccess(msg) => {
        state.darwin_success += 1;
        println!("\r\x1b[2K  {} {}", Paint::new(ICON_SUCCESS).fg(GREEN), msg);
      },
      LogLine::SectionHeader(section) => {
        println!("\n{} {}", Paint::new(ICON_ARROW).fg(PURPLE).bold(), section.trim());
      },
      LogLine::Warning(warn) => {
        if warn.contains("not installed") {
          state.brew_missing += 1;
        }
        println!(
          "\r\x1b[2K  {} Warning: {}",
          Paint::new(ICON_WARNING).fg(YELLOW),
          warn
        );
      },
      LogLine::Other(other) => {
        if other.contains("Starting Home Manager activation") {
          println!("\r\x1b[2K");
        } else if other.contains("Error:") || other.contains("failed") {
          println!(
            "\r\x1b[2K  {} {}",
            Paint::new(ICON_WARNING).fg(RED),
            Paint::new(other).fg(RED)
          );
        } else if !other.is_empty() {
          println!("\r\x1b[2K  {}", Paint::new(other).dim());
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

  // Print a final summary at the end
  println!(
    "\n{} Activation Summary",
    Paint::new(ICON_SUCCESS).fg(GREEN).bold()
  );
  if state.brew_using > 0
    || state.brew_installing > 0
    || state.brew_upgrading > 0
  {
    println!(
      "  {} Homebrew: {} installed, {} upgraded, {} working fine",
      Paint::new(ICON_ARROW).fg(PURPLE),
      state.brew_installing,
      state.brew_upgrading,
      state.brew_using
    );
  }
  if state.hm_count > 0 {
    println!(
      "  {} Home Manager: {} modules activated",
      Paint::new(ICON_ARROW).fg(PURPLE),
      state.hm_count
    );
  }
  if state.darwin_success > 0 {
    println!(
      "  {} Darwin: {} system settings applied",
      Paint::new(ICON_ARROW).fg(PURPLE),
      state.darwin_success
    );
  }
  if state.flake_updates > 0 {
    println!(
      "  {} Flake: {} inputs updated",
      Paint::new(ICON_ARROW).fg(PURPLE),
      state.flake_updates
    );
  }

  if state.brew_installing == 0
    && state.brew_upgrading == 0
    && state.hm_count == 0
    && state.darwin_success == 0
    && state.flake_updates == 0
  {
    println!(
      "  {} Configuration is already up to date",
      Paint::new(ICON_SUCCESS).fg(GREEN)
    );
  }

  Ok(())
}
