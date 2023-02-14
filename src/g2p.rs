use anyhow::{anyhow, Result};
use rustfst::algorithms::compose::compose;
use rustfst::prelude::*;
use rustfst::utils::decode_linear_fst;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration parameters for the g2p
#[derive(Debug)]
pub struct Config {
    /// Grapheme separator
    pub gsep: String,
    /// Phoneme skip marker
    pub skip: String,
    /// Write the output FSTs for debugging
    pub write_fsts: bool,
}

/// Grapheme to phoneme converter
#[derive(Debug)]
pub struct G2P {
    /// Configuration
    pub config: Config,
    /// Model (just a WFST actually)
    model: VectorFst<TropicalWeight>,
    /// Maximum size of input clusters
    imax: u8,
    /// Input symbol table
    isyms: Arc<SymbolTable>,
    /// Output symbol table
    osyms: Arc<SymbolTable>,
    /// Reverse mapping of input symbol clusters
    inv_imap: HashMap<Vec<Label>, Label>,
    /// Mapping of output symbol clusters
    omap: HashMap<Label, Vec<Label>>,
    // Other mappings are not used!
}

impl G2P {
    pub fn new(config: Config, mut model: VectorFst<TropicalWeight>) -> Result<G2P> {
        let isyms = Arc::clone(
            model
                .input_symbols()
                .ok_or(anyhow!("No input symbol table"))?,
        );
        let osyms = Arc::clone(
            model
                .output_symbols()
                .ok_or(anyhow!("No output symbol table"))?,
        );
        tr_sort(&mut model, ILabelCompare {});
        let (imax, _imap, inv_imap) = Self::load_clusters(&isyms)?;
        let (_omax, omap, _inv_omap) = Self::load_clusters(&osyms)?;
        Ok(G2P {
            config,
            model,
            isyms,
            osyms,
            imax,
            inv_imap,
            omap,
        })
    }

    fn load_clusters(
        syms: &SymbolTable,
    ) -> Result<(u8, HashMap<Label, Vec<Label>>, HashMap<Vec<Label>, Label>)> {
        let mut clusters = HashMap::<Label, Vec<Label>>::new();
        let mut invclusters = HashMap::<Vec<Label>, Label>::new();
        let tie = syms
            .get_symbol(1)
            .ok_or(anyhow!("Cluster separator not found in symbol table"))?; // FIXME: stupid magic
        let mut maxlen = 1;
        for i in 2..syms.len() as u32 {
            let sym = syms
                .get_symbol(i)
                .expect("Symbol table lies about its size");
            let cluster: Result<Vec<Label>, _> = sym
                .split(tie)
                .map(|s| {
                    syms.get_label(s)
                        .ok_or(anyhow!("Symbol {} not found in cluster {}", s, sym))
                })
                .collect();
            let cluster = cluster?;
            maxlen = max(maxlen, cluster.len());
            clusters.insert(i, cluster.clone());
            invclusters.insert(cluster, i);
        }
        let maxlen: u8 = maxlen.try_into()?;
        Ok((maxlen, clusters, invclusters))
    }

    fn entry_to_fsa(
        &self,
        word: &Vec<Label>,
        maxlen: u8,
        invmap: &HashMap<Vec<Label>, Label>,
    ) -> Result<VectorFst<TropicalWeight>> {
        let mut fsa = VectorFst::<TropicalWeight>::new();
        let maxlen = maxlen as u32;
        fsa.add_state();
        fsa.set_start(0)?;
        let nsyms: u32 = word.len().try_into()?;
        for i in 0..nsyms {
            fsa.add_state();
            let label = word[i as usize];
            fsa.add_tr(
                i,
                Tr::<TropicalWeight>::new(label, label, TropicalWeight::one(), i + 1),
            )?;
            for j in 2..=min(maxlen, nsyms - i) {
                let subv_start = i as usize; // OMG STFU RUSTC
                let subv_end = (i + j) as usize;
                let subv = &word[subv_start..subv_end];
                if let Some(&label) = invmap.get(subv) {
                    fsa.add_tr(
                        i,
                        Tr::<TropicalWeight>::new(label, label, TropicalWeight::one(), i + j),
                    )?;
                }
            }
        }
        fsa.set_final(nsyms, TropicalWeight::one())?;
        Ok(fsa)
    }

    pub fn g2p(&self, word: &str) -> Result<(Vec<&str>, f32)> {
        let maybe_syms: Result<Vec<Label>, _> = word
            .split(&self.config.gsep)
            .filter(|s| !s.is_empty())
            .map(|s| {
                self.isyms
                    .get_label(s)
                    .ok_or(anyhow!("Input symbol {} not found", s))
            })
            .collect();

        let mut fst = self.entry_to_fsa(&maybe_syms?, self.imax, &self.inv_imap)?;
        fst.set_input_symbols(Arc::clone(&self.isyms));
        fst.set_output_symbols(Arc::clone(&self.isyms));
        if self.config.write_fsts {
            fst.write(word.to_owned() + ".fst")?;
        }

        // WTF
        let fst: VectorFst<TropicalWeight> = compose::<
            TropicalWeight,
            VectorFst<TropicalWeight>,
            VectorFst<TropicalWeight>,
            _,
            _,
            _,
        >(fst, &self.model)?;
        if self.config.write_fsts {
            fst.write(word.to_owned() + ".lat.fst")?;
        }
        let fst: VectorFst<TropicalWeight> = shortest_path(&fst)?;
        if self.config.write_fsts {
            fst.write(word.to_owned() + ".path.fst")?;
        }
        let path = decode_linear_fst(&fst)?;
        // In Lisp or Python, this would easy in functional style, in
        // Rust, because of the incomprehensible type inference... NO.
        let mut wtf_rustc = Vec::<&str>::new();
        for label in path.olabels {
            if let Some(cluster) = self.omap.get(&label) {
                for &l in cluster {
                    // We should never have unknown labels in a cluster!
                    let sym = self
                        .osyms
                        .get_symbol(l)
                        .expect("Cluster has unknown labels");
                    wtf_rustc.push(sym);
                }
            } else if let Some(sym) = self.osyms.get_symbol(label) {
                wtf_rustc.push(sym);
            } else {
                // On the other hand the model might contain unknown labels
                return Err(anyhow!("Output label {} not found in model", label));
            }
        }
        let phones = wtf_rustc
            .into_iter()
            .filter(|&s| s != self.config.skip)
            .collect();
        Ok((phones, *path.weight.value()))
    }
}
