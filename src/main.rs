extern crate cairo;
extern crate conllx;
extern crate dot;
#[macro_use]
extern crate error_chain;
extern crate getopts;
extern crate gtk;
extern crate petgraph;
extern crate rsvg;
extern crate stdinout;

use std::cell::RefCell;
use std::env::args;
use std::process;
use std::rc::Rc;

use getopts::Options;
use gtk::prelude::*;
use gtk::{PolicyType, Viewport};
use stdinout::{Input, OrExit};

mod error;

mod graph;
use graph::{sentence_to_graph, DependencyGraph};

mod widgets;
use widgets::DependencyTreeWidget;

const ZOOM_IN_KEY: u32 = 61;
const ZOOM_OUT_KEY: u32 = 45;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] EXPR [INPUT_FILE]", program);
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

    let sentence = reader.into_iter().next().unwrap();
    let sentence = sentence.or_exit("Cannot read sentence", 1);

    let graph = sentence_to_graph(&sentence, false);

    gtk::init().or_exit("Failed to initialize GTK", 1);

    create_gui(800, 600, &graph);

    gtk::main();
}

pub fn create_gui(width: i32, height: i32, graph: &DependencyGraph) {
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("conllx-view");
    window.set_border_width(10);

    let dep_widget = Rc::new(RefCell::new(DependencyTreeWidget::new()));
    dep_widget
        .borrow_mut()
        .set_graph(graph)
        .or_exit("Error setting graph", 1);
    let dep_widget_clone = dep_widget.clone();

    window.connect_key_press_event(move |_, key_event| {
        println!("key: {}", key_event.get_keyval());
        match key_event.get_keyval() {
            ZOOM_IN_KEY => {
                let mut widget_mut = dep_widget_clone.borrow_mut();
                widget_mut.zoom_in();
                widget_mut.queue_draw();
            }
            ZOOM_OUT_KEY => {
                let mut widget_mut = dep_widget_clone.borrow_mut();
                widget_mut.zoom_out();
                widget_mut.queue_draw();
            }
            _ => (),
        }
        Inhibit(false)
    });

    let viewport = Viewport::new(None, None);
    viewport.add(dep_widget.borrow().inner());

    let scroll = gtk::ScrolledWindow::new(None, None);
    scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    scroll.add(&viewport);

    window.set_default_size(width, height);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    window.add(&scroll);
    window.show_all();
}
