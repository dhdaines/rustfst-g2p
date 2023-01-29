mod aligner;
use aligner::{Aligner, Params};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let mut aligner = Aligner::new(Params::parse());
    aligner.load_dictionary()?;
    eprintln!("Starting EM...");
    aligner.maximization()?;
    for i in 1..=aligner.params.iter {
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
