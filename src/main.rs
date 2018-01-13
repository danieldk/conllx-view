extern crate conllx;
extern crate dot;
extern crate getopts;
extern crate petgraph;
extern crate stdinout;

use std::env::args;
use std::io::{Read, Write};
use std::process::{self, Command, Stdio};

use dot::render;
use getopts::Options;
use stdinout::{Input, OrExit};

mod graph;
use graph::sentence_to_graph;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] EXPR [INPUT_FILE]", program);
    print!("{}", opts.usage(&brief));
}

fn dot_to_svg(dot: &[u8]) -> String {
    // FIXME: bind against C library?

    // Spawn Graphviz dot for rendering SVG (Fixme: bind against C library?).
    let process = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .or_exit("Couldn't spawn dot", 1);

    process.stdin.unwrap().write_all(dot).or_exit("Cannot write to dot stdin", 1);

    let mut svg = String::new();
    process.stdout.unwrap().read_to_string(&mut svg).or_exit("Cannot read dot stdout", 1);

    svg
}

fn main() {
    let args: Vec<String> = args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optopt(
        "l",
        "layer",
        "layer: form, lemma, cpos, pos, headrel, or pheadrel (default: form)",
        "LAYER",
    );
    let matches = opts.parse(&args[1..])
        .or_exit("Could not parse command-line arguments", 1);

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    if matches.free.len() > 1 {
        print_usage(&program, opts);
        process::exit(1);
    }

    let input = Input::from(matches.free.get(0));
    let reader = conllx::Reader::new(input.buf_read().or_exit("Cannot open input for reading", 1));

    for sentence in reader {
        let sentence = sentence.or_exit("Cannot read sentence", 1);

        let graph = sentence_to_graph(&sentence, false);
        //let simplified_graph = graph.map(|_, n| n.token.form(), |_, e| e.unwrap());

        //println!("{:?}", Dot::with_config(&simplified_graph, &[]));

        let mut dot = Vec::new();
        render(&graph, &mut dot).or_exit("Error writing dot output", 1);
        let svg = dot_to_svg(&dot);

        println!("{}", svg);

        return;
    }
}
