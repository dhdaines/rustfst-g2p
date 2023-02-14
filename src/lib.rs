use rustfst::fst_impls::VectorFst;
use rustfst::semirings::{LogWeight, TropicalWeight};

pub mod align;
pub mod g2p;
pub mod train;

type StdVectorFst = VectorFst<TropicalWeight>;
type LogVectorFst = VectorFst<LogWeight>;
