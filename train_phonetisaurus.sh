#!/bin/sh

TMPDIR=p11s
mkdir -p $TMPDIR
phonetisaurus-align --iter=10 --input=testdata/librispeech.train.sample \
                    --ofile=$TMPDIR/train.aligned --seq1_del=false --seq2_del=true \
                    --seq1_max=2 --seq2_max=2 --grow=false
ngramsymbols $TMPDIR/train.aligned $TMPDIR/train.syms
farcompilestrings --symbols=$TMPDIR/train.syms --keep_symbols $TMPDIR/train.aligned \
    | ngramcount --order=5 | ngrammake --method=kneser_ney - $TMPDIR/train.mod
ngramprint -ARPA $TMPDIR/train.mod $TMPDIR/train.arpa
phonetisaurus-arpa2wfst --lm=$TMPDIR/train.arpa --ofile=$TMPDIR/model.fst
fstprint $TMPDIR/model.fst > $TMPDIR/model.fst.txt
cut -d' ' -f1 testdata/librispeech.test.sample > $TMPDIR/test.words
phonetisaurus-g2pfst --model=$TMPDIR/model.fst --wordlist=$TMPDIR/test.words > $TMPDIR/test.hyp
python calculateER.py --hyp $TMPDIR/test.hyp --ref testdata/librispeech.test.sample 
