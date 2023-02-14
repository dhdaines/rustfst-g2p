use anyhow::{anyhow, Result};
use rustfst::algorithms::shortest_path;
use rustfst::algorithms::weight_converters::SimpleWeightConverter;
use rustfst::prelude::*;
use rustfst::semirings::DivideType::DivideAny;
use rustfst::utils::decode_linear_fst;
use std::cmp::min;
use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

/// Configuration parameters for the aligner
#[derive(Debug)]
pub struct Config {
    /// Maximum length of an input multi-token
    pub seq1_max: u8,
    /// Maximum length of an output multi-token
    pub seq2_max: u8,
    /// Allow deletion of input tokens
    pub seq1_del: bool,
    /// Allow deletion of output tokens
    pub seq2_del: bool,
    /// Restrict to N-1 and 1-M alignments
    pub restrict: bool,
    /// Multi-token separator for input tokens
    pub seq1_sep: String,
    /// Multi-token separator for output tokens
    pub seq2_sep: String,
    /// Token used to separate input-output subsequences in the g2p model
    pub s1s2_sep: String,
    /// Epsilon symbol
    pub eps: String,
    /// Skip token used to represent null transitions.  Distinct from epsilon
    pub skip: String,
    /// Sequence one input separator
    pub s1_char_delim: String,
    /// Sequence two input separator
    pub s2_char_delim: String,
}

/// Grapheme to phoneme aligner
#[derive(Debug)]
pub struct Aligner {
    pub config: Config,
    isyms: SymbolTable,
    fsas: Vec<VectorFst<LogWeight>>,
    alignment_model: HashMap<Label, LogWeight>,
    prev_alignment_model: HashMap<Label, LogWeight>,
    total: LogWeight,
    prev_total: LogWeight,
}

impl Aligner {
    /// Construct a new aligner with the given configuration
    pub fn new(config: Config) -> Aligner {
        let mut isyms = SymbolTable::empty();
        let fsas = Vec::<VectorFst<LogWeight>>::new();
        let alignment_model = HashMap::<Label, LogWeight>::new();
        let prev_alignment_model = HashMap::<Label, LogWeight>::new();
        let total = LogWeight::zero();
        let prev_total = LogWeight::zero();
        isyms.add_symbol(&config.eps);
        isyms.add_symbol(&config.skip);
        // use of _ here is "dangerous", apparently
        isyms.add_symbol(config.seq1_sep.as_str().to_owned() + "_" + config.seq2_sep.as_str());
        isyms.add_symbol(&config.s1s2_sep);
        // not sure what this is for but we will add it to have the same ids
        let model_config = format!(
            "{}_{}_{}_{}",
            config.seq1_del, config.seq2_del, config.seq1_max, config.seq2_max
        );
        isyms.add_symbol(model_config);
        Aligner {
            config,
            isyms,
            fsas,
            alignment_model,
            prev_alignment_model,
            total,
            prev_total,
        }
    }
    /// Initialize alignment from a pronunciation dictionary in text format
    pub fn load_dictionary(&mut self, input: &PathBuf) -> Result<()> {
        let fh = File::open(input)?;
        let reader = BufReader::new(fh);
        for spam in reader.lines().flatten() {
            let fields: Vec<&str> = spam.trim().split('\t').filter(|s| !s.is_empty()).collect();
            if fields.len() != 2 {
                return Err(anyhow!(
                    "Malformed line (must separate in/out with TAB): {}",
                    spam
                ));
            }
            let seq1: Vec<&str> = fields[0]
                .split(&self.config.s1_char_delim)
                .filter(|s| !s.is_empty())
                .collect();
            let seq2: Vec<&str> = fields[1]
                .split(&self.config.s2_char_delim)
                .filter(|s| !s.is_empty())
                .collect();
            // Just ignore failed alignments
            if let Err(err) = self.add_entry(&seq1, &seq2) {
                eprintln!("Ignoring: {}", err);
            }
        }
        Ok(())
    }

    fn add_entry(&mut self, seq1: &Vec<&str>, seq2: &Vec<&str>) -> Result<()> {
        let cli = &self.config;
        let skip = cli.skip.as_str();
        let s1s2_sep = cli.s1s2_sep.as_str();
        let seq1_sep = cli.seq1_sep.as_str();
        let seq2_sep = cli.seq2_sep.as_str();
        let mut fsa = VectorFst::<LogWeight>::new();
        for i in 0..=seq1.len() {
            for j in 0..=seq2.len() {
                let istate = fsa.add_state();
                assert!(istate as usize == i * (seq2.len() + 1) + j);
                if cli.seq1_del {
                    for jl in 1..=min(cli.seq2_max as usize, seq2.len() - j) {
                        let subseq2 = &seq2[j..j + jl].join(seq2_sep);
                        let isymname = skip.to_owned() + s1s2_sep + subseq2;
                        let isym = self.isyms.add_symbol(&isymname);
                        // Note: this state doesn't exist yet... ugh!
                        // FIXME: catch and report overflow, don't panic
                        let ostate: u32 = (i * (seq2.len() + 1) + (j + jl)).try_into().unwrap();
                        let tr = Tr::<LogWeight>::new(isym, isym, LogWeight::new(99.0), ostate);
                        fsa.add_tr(istate, tr)?;
                    }
                }
                if cli.seq2_del {
                    for ik in 1..=min(cli.seq1_max as usize, seq1.len() - i) {
                        let subseq1 = &seq1[i..i + ik].join(seq1_sep);
                        let isymname = subseq1.to_owned() + s1s2_sep + skip;
                        let isym = self.isyms.add_symbol(&isymname);
                        // Note: this state doesn't exist yet... ugh!
                        // FIXME: catch and report overflow, don't panic
                        let ostate: u32 = ((i + ik) * (seq2.len() + 1) + j).try_into().unwrap();
                        let tr = Tr::<LogWeight>::new(isym, isym, LogWeight::new(99.0), ostate);
                        fsa.add_tr(istate, tr)?;
                    }
                }
                for ik in 1..=min(cli.seq1_max as usize, seq1.len() - i) {
                    for jl in 1..=min(cli.seq2_max as usize, seq2.len() - j) {
                        let s1 = &seq1[i..i + ik].join(seq1_sep);
                        let s2 = &seq2[j..j + jl].join(seq2_sep);
                        if cli.restrict && jl > 1 && ik > 1 {
                            continue;
                        }
                        let isymname = s1.to_owned() + s1s2_sep + s2;
                        let isym = self.isyms.add_symbol(&isymname);
                        let ostate: u32 =
                            ((i + ik) * (seq2.len() + 1) + (j + jl)).try_into().unwrap();
                        let tr = Tr::<LogWeight>::new(isym, isym, LogWeight::one(), ostate);
                        fsa.add_tr(istate, tr)?;
                    }
                }
            }
        }
        fsa.set_start(0)?;
        let final_state: u32 = ((seq1.len() + 1) * (seq2.len() + 1) - 1)
            .try_into()
            .unwrap();
        fsa.set_final(final_state, LogWeight::one())?;
        // unless seq1_del && seq2_del, we will have unconnected states
        if !(self.config.seq1_del && self.config.seq2_del) {
            connect(&mut fsa)?;
        }
        if fsa.num_states() == 0 {
            return Err(anyhow!(
                "Alignment failed from {} to {}",
                seq1.join(seq1_sep),
                seq2.join(seq2_sep)
            ));
        }
        for q in fsa.states_iter() {
            for arc in fsa.get_trs(q)?.trs() {
                // Thanks, Clippy!
                if let Vacant(e) = self.prev_alignment_model.entry(arc.ilabel) {
                    e.insert(arc.weight);
                } else {
                    let weight = self.prev_alignment_model.get_mut(&arc.ilabel).unwrap();
                    weight.plus_assign(arc.weight)?;
                }
                self.total.plus_assign(arc.weight)?;
            }
        }
        self.fsas.push(fsa);
        Ok(())
    }

    /// E-step of EM alignment
    pub fn expectation(&mut self) -> Result<()> {
        for fsa in &self.fsas {
            let alpha = shortest_distance(fsa, false)?;
            let beta = shortest_distance(fsa, true)?;
            for q in fsa.states_iter() {
                for arc in fsa.get_trs(q)?.trs() {
                    let gamma = alpha[q as usize]
                        .times(&arc.weight)?
                        .times(&beta[arc.nextstate as usize])?
                        .divide(&beta[0], DivideAny)?;
                    if !gamma.value().is_nan() {
                        // Update prev_alignment_model which will be
                        // used to calculate the M-step below
                        let weight = self
                            .prev_alignment_model
                            .entry(arc.ilabel)
                            .or_insert_with(LogWeight::zero);
                        weight.plus_assign(gamma)?;
                        self.total.plus_assign(gamma)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// M-step of EM alignment
    pub fn maximization(&mut self) -> Result<f32> {
        let change = (self.total.value() - self.prev_total.value()).abs();
        // Apparently, "results are inconclusive" for the hideous
        // temporary-file-based hack that Phonetisaurus does here, so
        // we will NOT do that, and will do plain old maximum
        // likelihood instead.
        self.prev_total = self.total;
        for (&label, weight) in self.prev_alignment_model.iter_mut() {
            let estimate = weight.divide(&self.total, DivideAny)?;
            self.alignment_model.insert(label, estimate);
            weight.set_value(*LogWeight::zero().value());
        }
        for fsa in self.fsas.iter_mut() {
            for q in fsa.states_iter() {
                // The mutable arc iteration API in rustfst is not great
                let mut trs = fsa.tr_iter_mut(q)?;
                for idx in 0..trs.len() {
                    // Avoid penalize_em for the moment
                    let label = trs[idx].ilabel;
                    let weight = self.alignment_model[&label];
                    trs.set_weight(idx, weight)?;
                }
            }
        }
        self.total = LogWeight::zero();
        Ok(change)
    }

    /// Print alignments found to standard output
    pub fn print_alignments(&self) -> Result<()> {
        let mut mapper = SimpleWeightConverter {};
        for fsa in &self.fsas {
            // Do not do any N-Best, forward-backward pruning, or any
            // other such nonsense which the Phonetisaurus code admits
            // is not very useful
            let tfsa: VectorFst<TropicalWeight> = weight_convert(fsa, &mut mapper)?;
            let tfsa: VectorFst<TropicalWeight> = shortest_path(&tfsa)?;
            let path = decode_linear_fst(&tfsa)?;
            // Handling undefined symbols with map/filter is much too hard
            let mut syms = Vec::<&str>::new();
            for label in path.ilabels {
                match self.isyms.get_symbol(label) {
                    None => return Err(anyhow!("Undefined label {}", label)),
                    Some(sym) => syms.push(sym),
                }
            }
            println!("{}", syms.join(" "));
        }
        Ok(())
    }
}
