# k-gen

## Random chromatic formula generator for improvisers.

This chromatic formulas generator is inspired by An Improviser's OS by Wayne Krantz —
possibly the most interesting approach to creative improvisation ever put together.
The book is available from [Wayne Krantz](https://waynekrantz.bandcamp.com/merch/wayne-krantz-an-improvisers-os-2nd-edition) directly.

A *formula* is any subset of the twelve chromatic functions that contains the root:
`1 b2 2 b3 3 4 b5 5 b6 6 b7 7`. There are 2048 of them, from `1` alone to the full
chromatic scale. `k-gen` draws random ones for you to practice with — optionally
spelled out as note names in a given key.

This functionality (and some more) is also a part of my other project: [Solitito](https://github.com/greblus/solitito) - Real-Time Polyphonic Guitar Trainer
(Formulas mode).

## Build

No dependencies, so either works:

```sh
cargo build --release        # target/release/k-gen
rustc -O -o k-gen src/main.rs
```

Prebuilt Linux and Windows binaries are attached to each
[release](../../releases).

## Usage

Usage:

```sh
$ k-gen --help
k-gen — random chromatic formula generator

USAGE:
    k-gen -n <NOTES> -c <COUNT> [OPTIONS]

ARGUMENTS:
    -n,  --notes <1-12>     notes per formula (the root counts)
    -c,  --count <N>        how many random formulas to print

OPTIONS:
    -m,  --must <FUNCS>     only draw formulas containing these functions,
                            e.g. -m "b3 b7" (repeatable)
    -nn, --notenames [KEY]  print note names under the functions;
                            optional key (e.g. -nn A, -nn Eb, -nn F#),
                            otherwise a random one per formula
    -s,  --seed <N>         random seed (reproducible output)
         --compact          no spaces (1b22b3 instead of 1 b2 2 b3)
    -h,  --help             this help

EXAMPLES:
    k-gen -n 5 -c 10
    k-gen -n 5 -c 10 -nn
    k-gen -n 4 -c 3 -nn Eb
    k-gen -n 6 -c 8 -m "b3 b7" -nn
```

## Examples

```sh
$ k-gen -n 5 -c 3
1 4 b5 6 b7
1 b2 2 b3 5
1 b3 3 b5 b6

$ k-gen -n 5 -c 2 -nn A
A: 1 4 b5 6  b7
   A D D# F# G

A: 1 b2 2 b3 5
   A A# B C  E

$ k-gen -n 6 -c 2 -m "b3 b7" -nn Eb
Eb: 1  b3 b5 b6 6 b7
    Eb Gb A  B  C Db

Eb: 1  b3 4  5  b6 b7
    Eb Gb Ab Bb B  Db
```

## Notes

Formulas are drawn without repetition. Ask for more than a group holds and you get
the whole group plus a note on stderr:

```
$ k-gen -n 12 -c 5
Note: the 12-note group holds only 1 formula — printing all of them.
1 b2 2 b3 3 4 b5 5 b6 6 b7 7
```

Note names are spelled from the major scale of the key, so `1 b3 5 b7` in Eb comes
out as Eb Gb Bb Db, and in B as B D F# A. Where that would produce something
awkward — a double flat, or Fb, Cb, E#, B# — the easier enharmonic equivalent is
used instead, spelled to match the key.

---

