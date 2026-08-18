//! k-gen — random chromatic formula generator.
//!
//! A formula is a subset of the 12 chromatic functions that always contains
//! the root ("1"). An n-note group therefore holds C(11, n-1) formulas.
//!
//! No external dependencies: builds with plain `rustc`.

use std::collections::HashSet;
use std::env;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const FUNCS: [&str; 12] = [
    "1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7",
];

/// For every function: (major scale degree 0..6, lowering in semitones).
/// Functions 1..7 are the degrees of the major scale; the remaining five
/// are written as flats.
const DEGREE: [(usize, i32); 12] = [
    (0, 0),  // 1
    (1, -1), // b2
    (1, 0),  // 2
    (2, -1), // b3
    (2, 0),  // 3
    (3, 0),  // 4
    (4, -1), // b5
    (4, 0),  // 5
    (5, -1), // b6
    (5, 0),  // 6
    (6, -1), // b7
    (6, 0),  // 7
];

const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
const LETTER_PITCH: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

const NAMES_FLAT: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B",
];
const NAMES_SHARP: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Keys used when none is given — one per pitch class, in the spelling
/// most commonly used in practice.
const KEY_POOL: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
];

// ----------------------------------------------------------------- PRNG

/// xorshift64* — good enough for shuffling, and dependency free.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // zero is a fixed point of xorshift
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform value in [0, n): rejects the tail so modulo stays unbiased.
    fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        let zone = u64::MAX - (u64::MAX % n);
        loop {
            let v = self.next_u64();
            if v < zone {
                return v % n;
            }
        }
    }
}

fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x1234_5678_9ABC_DEF0)
}

// ------------------------------------------------------- building a group

/// Every n-note formula as a bit mask (bit i = FUNCS[i]).
/// Ordered lexicographically by chromatic position.
fn group(n: usize) -> Vec<u16> {
    let mut out = Vec::new();
    let mut pick: Vec<usize> = Vec::with_capacity(n - 1);
    build(1, n - 1, &mut pick, &mut out);
    out
}

fn build(start: usize, left: usize, pick: &mut Vec<usize>, out: &mut Vec<u16>) {
    if left == 0 {
        let mut mask: u16 = 1; // the root is always present
        for &i in pick.iter() {
            mask |= 1 << i;
        }
        out.push(mask);
        return;
    }
    // leave enough room to still pick `left` functions from 1..12
    for i in start..=(12 - left) {
        pick.push(i);
        build(i + 1, left - 1, pick, out);
        pick.pop();
    }
}

fn functions_of(mask: u16) -> Vec<usize> {
    (0..12).filter(|i| mask & (1 << i) != 0).collect()
}

fn parse_function(s: &str) -> Result<usize, String> {
    FUNCS
        .iter()
        .position(|&f| f == s)
        .ok_or_else(|| format!("unknown function: '{s}' (use 1, b2, 2, b3, 3, 4, b5, 5, b6, 6, b7, 7)"))
}

/// Parses a filter like "b3 b7" or "b3,b7" into a bit mask of required
/// functions. The root is implicit and always included.
fn parse_filter(s: &str) -> Result<u16, String> {
    let mut mask: u16 = 1;
    let mut any = false;
    for tok in s.split(|c: char| c.is_whitespace() || c == ',') {
        if tok.is_empty() {
            continue;
        }
        mask |= 1 << parse_function(tok)?;
        any = true;
    }
    if !any {
        return Err("-m needs at least one function, e.g. -m \"b3 b7\"".to_string());
    }
    Ok(mask)
}

fn render_mask(mask: u16) -> String {
    functions_of(mask)
        .iter()
        .map(|&i| FUNCS[i])
        .collect::<Vec<_>>()
        .join(" ")
}

// -------------------------------------------------------------------- keys

#[derive(Clone, Copy)]
struct Key {
    letter: usize, // index into LETTERS
    acc: i32,      // -1 flat, 0 natural, +1 sharp
    sharps: bool,  // preferred spelling when simplifying
}

impl Key {
    fn pitch(&self) -> i32 {
        (LETTER_PITCH[self.letter] + self.acc).rem_euclid(12)
    }

    fn name(&self) -> String {
        let a = match self.acc {
            -1 => "b",
            1 => "#",
            _ => "",
        };
        format!("{}{}", LETTERS[self.letter], a)
    }
}

fn parse_key(s: &str) -> Result<Key, String> {
    let mut ch = s.chars();
    let head = ch
        .next()
        .ok_or_else(|| "empty key name".to_string())?
        .to_ascii_uppercase();
    let letter = LETTERS
        .iter()
        .position(|&l| l == head)
        .ok_or_else(|| format!("unknown key: '{s}' (use A-G, e.g. Eb, F#)"))?;

    let tail: String = ch.collect();
    let acc = match tail.as_str() {
        "" => 0,
        "#" | "♯" => 1,
        "b" | "B" | "♭" => -1,
        _ => return Err(format!("unknown accidental in '{s}'")),
    };

    Ok(Key {
        letter,
        acc,
        // sharp spelling for sharp keys, flats everywhere else
        sharps: acc == 1 || (acc == 0 && matches!(head, 'G' | 'D' | 'A' | 'E' | 'B')),
    })
}

fn accidental(n: i32) -> Option<&'static str> {
    match n {
        -2 => Some("bb"),
        -1 => Some("b"),
        0 => Some(""),
        1 => Some("#"),
        2 => Some("##"),
        _ => None,
    }
}

/// Note name for a function in a given key.
///
/// The letter comes from the major scale degree, then accidentals are added.
/// When that yields a name outside the twelve practical ones (double
/// accidentals, Fb, Cb, E#, B#), the enharmonic equivalent is used instead,
/// spelled to match the key.
fn note_name(key: &Key, func: usize) -> String {
    let (deg, alt) = DEGREE[func];
    let letter = (key.letter + deg) % 7;
    let target = (key.pitch() + MAJOR[deg]).rem_euclid(12);

    // distance from the letter's natural pitch, folded into -6..6
    let mut diff = (target - LETTER_PITCH[letter]).rem_euclid(12);
    if diff > 6 {
        diff -= 12;
    }
    let acc = diff + alt;

    let table = if key.sharps { NAMES_SHARP } else { NAMES_FLAT };
    let pc = (target + alt).rem_euclid(12) as usize;

    match accidental(acc) {
        Some(a) => {
            let by_degree = format!("{}{}", LETTERS[letter], a);
            if table.contains(&by_degree.as_str()) {
                by_degree
            } else {
                table[pc].to_string()
            }
        }
        // more than two accidentals — always simplify
        None => table[pc].to_string(),
    }
}

// --------------------------------------------------------------- arguments

struct Args {
    notes: usize,
    count: usize,
    seed: Option<u64>,
    compact: bool,
    /// `Some(None)` = -nn with no key (a random one per formula),
    /// `Some(Some(k))` = -nn with a key, `None` = no -nn.
    notenames: Option<Option<Key>>,
    /// Functions every drawn formula must contain.
    required: u16,
}

fn usage() -> String {
    "\
k-gen — random chromatic formula generator

USAGE:
    k-gen -n <NOTES> -c <COUNT> [OPTIONS]

ARGUMENTS:
    -n,  --notes <1-12>     notes per formula (the root counts)
    -c,  --count <N>        how many random formulas to print

OPTIONS:
    -m,  --must <FUNCS>     only draw formulas containing these functions,
                            e.g. -m \"b3 b7\" (repeatable)
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
    k-gen -n 6 -c 8 -m \"b3 b7\" -nn"
        .to_string()
}

fn parse_num(flag: &str, val: Option<&String>) -> Result<u64, String> {
    let raw = val.ok_or_else(|| format!("{flag} needs a value"))?;
    raw.parse::<u64>()
        .map_err(|_| format!("invalid value for {flag}: '{raw}'"))
}

fn parse_args() -> Result<Args, String> {
    let argv: Vec<String> = env::args().skip(1).collect();

    let mut notes: Option<u64> = None;
    let mut count: Option<u64> = None;
    let mut seed: Option<u64> = None;
    let mut compact = false;
    let mut notenames: Option<Option<Key>> = None;
    let mut required: u16 = 1;

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "-n" | "--notes" => {
                notes = Some(parse_num("-n", argv.get(i + 1))?);
                i += 1;
            }
            "-c" | "--count" => {
                count = Some(parse_num("-c", argv.get(i + 1))?);
                i += 1;
            }
            "-s" | "--seed" => {
                seed = Some(parse_num("-s", argv.get(i + 1))?);
                i += 1;
            }
            "-m" | "--must" => {
                let raw = argv
                    .get(i + 1)
                    .ok_or_else(|| "-m needs a list of functions, e.g. -m \"b3 b7\"".to_string())?;
                required |= parse_filter(raw)?;
                i += 1;
            }
            "-nn" | "--notenames" => {
                // the key is optional: only take the next argument if it
                // exists and is not another option
                match argv.get(i + 1) {
                    Some(next) if !next.starts_with('-') => {
                        notenames = Some(Some(parse_key(next)?));
                        i += 1;
                    }
                    _ => notenames = Some(None),
                }
            }
            "--compact" => compact = true,
            other => return Err(format!("unknown argument: '{other}'\n\n{}", usage())),
        }
        i += 1;
    }

    let notes = notes.ok_or_else(|| format!("missing required option -n\n\n{}", usage()))?;
    let count = count.ok_or_else(|| format!("missing required option -c\n\n{}", usage()))?;

    if !(1..=12).contains(&notes) {
        return Err(format!("a formula holds 1 to 12 notes (got {notes})"));
    }
    if count == 0 {
        return Err("-c must be greater than 0".to_string());
    }

    let needed = required.count_ones() as u64;
    if needed > notes {
        return Err(format!(
            "the filter needs at least {} notes ({}), but -n is {}",
            needed,
            render_mask(required),
            notes
        ));
    }

    Ok(Args {
        notes: notes as usize,
        count: count as usize,
        seed,
        compact,
        notenames,
        required,
    })
}

// -------------------------------------------------------------------- output

fn plural(n: usize) -> &'static str {
    if n == 1 {
        "formula"
    } else {
        "formulas"
    }
}

fn print_plain(funcs: &[usize], compact: bool) {
    let parts: Vec<&str> = funcs.iter().map(|&i| FUNCS[i]).collect();
    println!("{}", parts.join(if compact { "" } else { " " }));
}

/// Two lines: the functions, and the note names lined up underneath.
fn print_with_names(funcs: &[usize], key: &Key) {
    let label = format!("{}:", key.name());
    let pad = " ".repeat(label.chars().count());

    let mut top = String::new();
    let mut bottom = String::new();
    for (n, &f) in funcs.iter().enumerate() {
        let fun = FUNCS[f];
        let note = note_name(key, f);
        // column width = the longer of the two strings
        let w = fun.chars().count().max(note.chars().count());
        if n > 0 {
            top.push(' ');
            bottom.push(' ');
        }
        top.push_str(&format!("{:<w$}", fun, w = w));
        bottom.push_str(&format!("{:<w$}", note, w = w));
    }

    println!("{} {}", label, top.trim_end());
    println!("{} {}", pad, bottom.trim_end());
    println!();
}

// ---------------------------------------------------------------------- main

fn main() -> ExitCode {
    // help is not an error: stdout, exit 0
    if env::args().skip(1).any(|a| a == "-h" || a == "--help") {
        println!("{}", usage());
        return ExitCode::SUCCESS;
    }

    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(1);
        }
    };

    let mut pool = group(args.notes);
    if args.required != 1 {
        pool.retain(|&m| m & args.required == args.required);
    }
    let total = pool.len();

    // how many we can actually print without repeating ourselves
    let take = args.count.min(total);
    if args.count > total {
        if args.required == 1 {
            eprintln!(
                "Note: the {}-note group holds only {} {} — printing all of them.",
                args.notes,
                total,
                plural(total)
            );
        } else {
            eprintln!(
                "Note: only {} {} in the {}-note group {} -m \"{}\" — printing all of them.",
                total,
                plural(total),
                args.notes,
                if total == 1 { "matches" } else { "match" },
                render_mask(args.required & !1)
            );
        }
    }

    let seed = args.seed.unwrap_or_else(time_seed);
    let mut rng = Rng::new(seed);

    // Fisher-Yates, but only for as many slots as we need
    for i in 0..take {
        let j = i + rng.below((total - i) as u64) as usize;
        pool.swap(i, j);
    }

    let mut seen = HashSet::new();
    for &mask in pool.iter().take(take) {
        debug_assert!(seen.insert(mask), "duplicate formula");
        let funcs = functions_of(mask);

        match args.notenames {
            Some(chosen) => {
                let key = chosen.unwrap_or_else(|| {
                    let pick = KEY_POOL[rng.below(KEY_POOL.len() as u64) as usize];
                    parse_key(pick).expect("KEY_POOL holds valid keys")
                });
                print_with_names(&funcs, &key);
            }
            None => print_plain(&funcs, args.compact),
        }
    }

    ExitCode::SUCCESS
}
