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
          // just extract the path, ignore attribute e.g. path#attr -> path
          let mut elems = flake.splitn(2, '#');
          elems.next().unwrap_or(".").to_string()
        } else {
          ".".to_string()
        };

        let mut cmd = Command::new("nix").arg("flake").arg("update").arg("--flake").arg(&flake_path);

        if let Some(inp) = input {
          cmd = cmd.arg(inp);
        }

        cmd.show_output(true).run()?;
        Ok(())
      }
    }
  }
}
