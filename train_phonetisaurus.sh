#!/bin/sh

TMPDIR=p11s
mkdir -p $TMPDIR
phonetisaurus-align --iter=5 --input=testdata/librispeech.train.sample \
                    --ofile=$TMPDIR/train.aligned --seq1_del=false --seq2_del=true \
                    --seq1_max=2 --seq2_max=2 --grow=false
ngramsymbols $TMPDIR/train.aligned $TMPDIR/train.syms
farcompilestrings --symbols=$TMPDIR/train.syms --keep_symbols $TMPDIR/train.aligned \
    | ngramcount --order=5 | ngrammake --method=kneser_ney - $TMPDIR/train.mod
ngramprint -ARPA $TMPDIR/train.mod $TMPDIR/train.arpa
phonetisaurus-arpa2wfst --lm=$TMPDIR/train.arpa --ofile=$TMPDIR/model.fst
fstprint $TMPDIR/model.fst > $TMPDIR/model.fst.txt
