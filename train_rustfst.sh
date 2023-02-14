#!/bin/sh

TMPDIR=rustfst
mkdir -p $TMPDIR
cargo run --release align --iter 10 \
      --seq1-del=false --seq2-del=true \
      --seq1-max=2 --seq2-max=2 \
      testdata/librispeech.train.sample > $TMPDIR/train.aligned
cargo run --release train --order=5 --write-fsts \
      $TMPDIR/train.aligned $TMPDIR/model.fst
cut -d' ' -f1 testdata/librispeech.test.sample > $TMPDIR/test.words
cargo run --release g2p $TMPDIR/model.fst $TMPDIR/test.words > $TMPDIR/test.hyp
python calculateER.py --hyp $TMPDIR/test.hyp --ref testdata/librispeech.test.sample 
