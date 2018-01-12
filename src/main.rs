extern crate conllx;
extern crate petgraph;
extern crate getopts;
extern crate stdinout;

use std::env::args;
use std::process;

use getopts::Options;
use petgraph::dot::{Dot, Config};
use stdinout::{Input, OrExit};

mod graph;
use graph::sentence_to_graph;

fn print_usage(program: &str, opts: Options) {
    let brief = format!(
        "Usage: {} [options] EXPR [INPUT_FILE]",
        program
    );
    print!("{}", opts.usage(&brief));
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
    let matches = opts.parse(&args[1..]).or_exit("Could not parse command-line arguments", 1);

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
        let simplified_graph = graph.map(|_, n| n.token.form(), |_, e| e.unwrap());

        println!("{:?}", Dot::with_config(&simplified_graph, &[]));

        return;
    }
}
