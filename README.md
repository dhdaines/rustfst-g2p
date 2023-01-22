rustg2p: the velociraptor of pronunciation generators
=====================================================

Back in days of yore, we used quaint devices known as weighted
finite-state transducers to accomplish a variety of natural language
and speech processing tasks.  Sure, they were not as sexy as
Transformers, but they had the advantage of being compact and
efficient and behaving in provably predictable ways (up to the limits
of floating-point math, that is).

There was/is a program called
[Phonetisaurus](https://github.com/AdolfVonKleist/Phonetisaurus) which
was/is quite popular for one particular task, creating automatic
pronunciation generators (a.k.a. grapheme-to-phoneme models) due to
being quite accurate and decently fast to train and apply.  It, in
turn, is based on [OpenFST](http://openfst.org), which is an extremely
impressive piece of free software with some serious problems:

- Every little version has a different API, and they are not binary
  compatible
- If you try to compile the latest version, it will brutally murder
  your computer unless you have a Core i11 42nd generation CPU and at
  least 666GB of RAM
- It is not so much a library as a bunch of interlinked header files,
  so anything that uses it also takes 50 times longer to compile than
  it should
- The resulting binaries are brontosaurically hugeungous

The venerable [AT&T FSM
library](https://www.openfst.org/twiki/bin/view/Contrib/FsmLibrary),
was more than good enough for all useful tasks, but sadly was never
released as free software, and thus, much as C++ "improved" on C, so
OpenFST and OpenGRM "improved" on the FSM and GRM libraries, with the
predictable results noted above.

But fear not!  For a kind soul has created
[rustfst](https://github.com/Garvys/rustfst), which fixes all of the
problems noted above, and is also quite a bit faster than OpenFST.
This seemed to me like a good opportunity to learn Rust, so in this
repository you will find (it is not quite ready yet) a rewrite of the
useful parts (there are many non-useful parts) of Phonetisaurus using
this faster, friendlier, and more efficient library.  Yes, it is also
faster.

I thought about calling it something awesome like "Vocaliraptor" and
maybe even getting the Dinosaur Comics guy to draw me a logo.  No,
actually, this idea never occurred to me, so it has a much more
prosaic name, but you can, of course, call it whatever you want.
