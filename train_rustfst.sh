#!/bin/sh

TMPDIR=rustfst
mkdir -p $TMPDIR
cargo run --release align --iter 5 \
      --seq1-del=false --seq2-del=true \
      --seq1-max=2 --seq2-max=2 \
      testdata/librispeech.train.sample > $TMPDIR/train.aligned
ngramsymbols $TMPDIR/train.aligned $TMPDIR/train.syms
farcompilestrings --symbols=$TMPDIR/train.syms --keep_symbols $TMPDIR/train.aligned \
    | ngramcount --order=5 | ngrammake --method=kneser_ney - $TMPDIR/train.mod
ngramprint -ARPA $TMPDIR/train.mod $TMPDIR/train.arpa
phonetisaurus-arpa2wfst --lm=$TMPDIR/train.arpa --ofile=$TMPDIR/model.fst
fstprint $TMPDIR/model.fst > $TMPDIR/model.fst.txt
