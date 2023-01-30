use anyhow::Result;
use clap::{Parser, Subcommand};
use rustfst_g2p::aligner::{Aligner, Config as AlignerConfig};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Aligns a dictionary
    Align {
        /// Input dictionary file
        input: PathBuf,
        /// Maximum length of an input multi-token
        #[arg(long, default_value_t = 2)]
        seq1_max: u8,
        /// Maximum length of an output multi-token
        #[arg(long, default_value_t = 2)]
        seq2_max: u8,
        /// Maximum number of EM iterations to perform
        #[arg(long, default_value_t = 11)]
        iter: u8,
        /// Allow deletion of input tokens
        #[arg(long, action = clap::ArgAction::Set, default_value_t = true)]
        seq1_del: bool,
        /// Allow deletion of output tokens
        #[arg(long, action = clap::ArgAction::Set, default_value_t = true)]
        seq2_del: bool,
        /// Restrict to N-1 and 1-M alignments
        #[arg(long, action = clap::ArgAction::Set, default_value_t = true)]
        restrict: bool,
        /// Multi-token separator for input tokens
        #[arg(long, default_value = "|")]
        seq1_sep: String,
        /// Multi-token separator for output tokens
        #[arg(long, default_value = "|")]
        seq2_sep: String,
        /// Token used to separate input-output subsequences in the g2p model
        #[arg(long, default_value = "}")]
        s1s2_sep: String,
        /// Epsilon symbol
        #[arg(long, default_value = "<eps>")]
        eps: String,
        /// Skip token used to represent null transitions.  Distinct from epsilon
        #[arg(long, default_value = "_")]
        skip: String,
        /// Sequence one input separator
        #[arg(long, default_value = "")]
        s1_char_delim: String,
        /// Sequence two input separator
        #[arg(long, default_value = " ")]
        s2_char_delim: String,
    },
    Train {},
    G2P {},
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // This is super gross... is there a better way?!?
    match cli.command {
        Commands::Align {
            input,
            iter,
            restrict,
            seq1_max,
            seq2_max,
            seq1_del,
            seq2_del,
            seq1_sep,
            seq2_sep,
            s1s2_sep,
            eps,
            skip,
            s1_char_delim,
            s2_char_delim,
        } => {
            let mut aligner = Aligner::new(AlignerConfig {
                restrict,
                seq1_max,
                seq2_max,
                seq1_del,
                seq2_del,
                seq1_sep,
                seq2_sep,
                s1s2_sep,
                eps,
                skip,
                s1_char_delim,
                s2_char_delim,
            });

            aligner.load_dictionary(&input)?;
            eprintln!("Starting EM...");
            aligner.maximization()?;
            for i in 1..=iter {
                aligner.expectation()?;
                let delta = aligner.maximization()?;
                eprintln!("Iteration: {} Change: {}", i, delta);
            }
            aligner.expectation()?;
            let delta = aligner.maximization()?;
            eprintln!("Last iteration: {}", delta);
            aligner.print_alignments()?;
            Ok(())
        }
        Commands::Train {} => Ok(()),
        Commands::G2P {} => Ok(()),
    }
}
