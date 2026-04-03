use clap::Parser;
use color_eyre::Result;
use nh_core::{
  checks::{FeatureRequirements, NoFeatures},
  command::Command,
};
use std::env;

#[derive(Parser, Debug)]
#[command(about = "Flake functionality")]
pub struct FlakeArgs {
  #[command(subcommand)]
  pub command: FlakeCommand,
}

#[derive(Parser, Debug)]
pub enum FlakeCommand {
  /// Update flake inputs
  Update {
    #[arg(help = "The target flake installable to update")]
    installable: Option<String>,

    /// Specific input to update
    #[arg(help = "The specific input to update")]
    input: Option<String>,
  },
}

impl FlakeArgs {
  #[must_use]
  pub fn get_feature_requirements(&self) -> Box<dyn FeatureRequirements> {
    Box::new(NoFeatures)
  }

  pub fn run(self) -> Result<()> {
    match self.command {
      FlakeCommand::Update { installable, input } => {
        let flake_path = if let Some(i) = installable {
          i
        } else if let Ok(flake) = env::var("NH_FLAKE") {
          let mut elems = flake.splitn(2, '#');
          elems.next().unwrap_or(".").to_string()
        } else {
          ".".to_string()
        };

        // Validate that the flake library exists
        let path = std::path::Path::new(&flake_path);
        if !path.exists() || !path.join("flake.nix").exists() {
           let msg = if env::var("NH_FLAKE").is_err() && flake_path == "." {
               "No flake found in current directory and NH_FLAKE is not set.\n\
                Please set NH_FLAKE to your dotfiles directory or run within a flake directory."
           } else {
               &format!("Flake not found at: {}\nEnsure this directory contains a flake.nix file.", flake_path)
           };
           color_eyre::eyre::bail!("{}", msg);
        }

        let mut cmd = Command::new("nix")
          .arg("flake")
          .arg("update")
          .arg("--flake")
          .arg(&flake_path);

        if let Some(inp) = input {
          cmd = cmd.arg(inp);
        }

        cmd.show_output(true).pretty(true).run()?;
        Ok(())
      },
    }
  }
}
