use crate::StdVectorFst;
use anyhow::{anyhow, Result};
use rustfst::prelude::*;
use rustfst::utils::acceptor;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

pub struct Config {
    /// Order of N-Grams
    pub order: u8,
    /// Write the output FSTs for debugging
    pub write_fsts: bool,
}

/// N-Gram trainer
pub struct NGram {
    /// Configuration
    pub config: Config,
    /// Linear FSAs for each input sequence
    pub inputs: Vec<StdVectorFst>,
    /// Symbol table for input alignments
    pub syms: SymbolTable,
}

impl NGram {
    pub fn new(config: Config) -> NGram {
        let inputs = Vec::<StdVectorFst>::new();
        let syms = SymbolTable::new();
        NGram {
            config,
            inputs,
            syms,
        }
    }

    /// Read aligned inputs
    pub fn load_alignments(&mut self, input: &PathBuf) -> Result<()> {
        let fh = File::open(input)?;
        let reader = BufReader::new(fh);
        // Where in the Rust documentation does it say that flatten()
        // skips over None values?
        for spam in reader.lines().flatten() {
            let labels: Vec<Label> = spam
                .trim()
                .split_whitespace()
                .map(|s| self.syms.add_symbol(s))
                .collect();
            // Will be topologically sorted, by definition
            let fsa: StdVectorFst = acceptor(&labels, TropicalWeight::one());
            self.inputs.push(fsa);
        }
        Ok(())
    }

    /// Count N-Grams and create fst
    pub fn get_ngram_counts(&self, alignments: &Vec<StdVectorFst>) -> Result<StdVectorFst> {
        let counts = StdVectorFst::new();
        Ok(counts)
    }

    /// Make modified Kneser-Ney model
    pub fn make_kn_model(&self, model: &StdVectorFst) -> Result<StdVectorFst> {
        let model = StdVectorFst::new();
        Ok(model)
    }

    /// Train an N-Gram model and convert to fst
    pub fn train(&self) -> Result<StdVectorFst> {
        // Collect counts
        let counts = self.get_ngram_counts(&self.inputs)?;
        // Create model
        let model = self.make_kn_model(&counts)?;

        Ok(model)
    }
}
