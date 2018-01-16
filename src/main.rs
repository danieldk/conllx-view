extern crate cairo;
extern crate conllx;
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
use gtk::PolicyType;
use stdinout::{Input, OrExit};

mod error;

mod graph;
use graph::sentence_to_graph;

#[macro_use]
mod macros;

mod model;
use model::StatefulTreebankModel;

mod widgets;
use widgets::{DependencyTreeWidget, SentenceWidget};

const NEXT_KEY: u32 = 110;
const PREVIOUS_KEY: u32 = 112;
const QUIT_KEY: u32 = 113;
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

    let dep_graph_iter = reader.into_iter().map(|sent| {
        let sent = sent.or_exit("Cannot read sentence", 1);
        sentence_to_graph(sent, false)
    });

    let treebank_model = StatefulTreebankModel::from_iter(dep_graph_iter);

    gtk::init().or_exit("Failed to initialize GTK", 1);

    create_gui(800, 600, treebank_model);

    gtk::main();
}

fn create_gui(width: i32, height: i32, treebank_model: StatefulTreebankModel) {
    let treebank_model = Rc::new(RefCell::new(treebank_model));

    let dep_widget = Rc::new(RefCell::new(DependencyTreeWidget::new()));
    let dep_widget_clone = dep_widget.clone();
    treebank_model.borrow_mut().connect_update(move |model| {
        if let Ok(handle) = model.handle() {
            dep_widget_clone.borrow_mut().update(handle);
        }
    });

    let scroll = gtk::ScrolledWindow::new(None, None);
    scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    scroll.add(dep_widget.borrow().inner());

    let sent_widget = Rc::new(RefCell::new(SentenceWidget::new()));
    let sent_widget_clone = sent_widget.clone();
    treebank_model.borrow_mut().connect_update(move |model| {
        sent_widget_clone.borrow_mut().update(model.sentence());
    });

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    vbox.pack_start(&scroll, true, true, 0);
    vbox.pack_start(sent_widget.borrow().inner(), false, false, 0);

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("conllx-view");
    window.set_border_width(10);

    setup_key_event_handling(&window, treebank_model.clone(), dep_widget.clone());

    window.set_default_size(width, height);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    window.add(&vbox);
    window.show_all();

    treebank_model.borrow_mut().first();
}

fn setup_key_event_handling(
    window: &gtk::Window,
    treebank_model: Rc<RefCell<StatefulTreebankModel>>,
    dep_widget: Rc<RefCell<DependencyTreeWidget>>,
) {
    window.connect_key_press_event(move |_, key_event| {
        println!("key: {}", key_event.get_keyval());
        match key_event.get_keyval() {
            NEXT_KEY => {
                treebank_model.borrow_mut().next();
            }
            PREVIOUS_KEY => {
                treebank_model.borrow_mut().previous();
            }
            QUIT_KEY => {
                gtk::main_quit();
            }
            ZOOM_IN_KEY => {
                let mut widget_mut = dep_widget.borrow_mut();
                widget_mut.zoom_in();
                widget_mut.queue_draw();
            }
            ZOOM_OUT_KEY => {
                let mut widget_mut = dep_widget.borrow_mut();
                widget_mut.zoom_out();
                widget_mut.queue_draw();
            }
            _ => (),
        }
        Inhibit(false)
    });
}
