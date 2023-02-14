use anyhow::Result;
use clap::{Parser, Subcommand};
use rustfst::prelude::*;
use rustfst_g2p::align::{Aligner, Config as AlignerConfig};
use rustfst_g2p::g2p::{Config as G2PConfig, G2P};
use rustfst_g2p::train::ngram::NGram;
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
    /// Performs grapheme-to-phoneme conversion on input(s)
    G2P {
        /// Path to trained model
        model: PathBuf,
        /// Grapheme separator
        #[arg(long, default_value = "")]
        gsep: String,
        /// Phoneme skip marker
        #[arg(long, default_value = "_")]
        skip: String,
        /// Write the output FSTs for debugging
        #[arg(long)]
        write_fsts: bool,
        /// Reverse input word
        #[arg(long)]
        reverse: bool,
        /// Print scores in output
        #[arg(long, action = clap::ArgAction::Set, default_value_t = true)]
        print_scores: bool,
        /// Default scores vals are negative logs
        #[arg(long, action = clap::ArgAction::Set, default_value_t = true)]
        nlog_probs: bool,
    },
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
        Commands::G2P {
            model,
            gsep,
            skip,
            write_fsts,
            reverse,
            print_scores,
            nlog_probs,
        } => {
            let model = VectorFst::<TropicalWeight>::read(&model)?;
            let g2p = G2P::new(
                G2PConfig {
                    gsep,
                    skip,
                    write_fsts,
                    reverse,
                    print_scores,
                    nlog_probs,
                },
                model,
            )?;
            println!("{:?}", g2p.g2p("USECASE"));
            Ok(())
        }
    }
}
