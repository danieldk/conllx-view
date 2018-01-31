extern crate cairo;
extern crate conllx;
#[macro_use]
extern crate error_chain;

extern crate getopts;
extern crate gio;
extern crate glib;
extern crate gtk;
extern crate itertools;
extern crate petgraph;
extern crate rsvg;
extern crate stdinout;

use std::cell::RefCell;
use std::env::args;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::process;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

use getopts::Options;
use gio::{ApplicationExt, ApplicationExtManual};
use gtk::prelude::*;
use gtk::PolicyType;
use rsvg::Handle;
use stdinout::{Input, OrExit};

mod error;
use error::*;

mod graph;
use graph::{DependencyGraph, Dot, Svg, Tikz, Tokens};

#[macro_use]
mod macros;

mod model;
use model::StatefulTreebankModel;

mod widgets;
use widgets::{DependencyTreeWidget, SentenceWidget};

const DOT_KEY: u32 = 100;
const NEXT_KEY: u32 = 110;
const PREVIOUS_KEY: u32 = 112;
const QUIT_KEY: u32 = 113;
const TIKZ_KEY: u32 = 116;
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

    let treebank_model = Arc::new(Mutex::new(StatefulTreebankModel::new()));

    gtk::init().or_exit("Failed to initialize GTK", 1);

    thread::spawn(clone!(treebank_model => move || {
        let reader = conllx::Reader::new(input.buf_read().or_exit("Cannot open input for reading", 1));

        let dep_graph_iter = reader.into_iter().map(|sent| {
            let sent = sent.or_exit("Cannot read sentence", 1);
            sent.into()
        });

        for graph in dep_graph_iter {
            treebank_model.lock().unwrap().push(graph);
        }
    }));

    let application =
        gtk::Application::new("eu.danieldk.conllx-view", gio::ApplicationFlags::empty())
            .expect("Initialization failed");

    application.connect_startup(move |app| {
        create_gui(app, 800, 600, treebank_model.clone());
    });

    application.connect_activate(|_| {});

    application.run(&args);
}

fn create_gui(
    application: &gtk::Application,
    width: i32,
    height: i32,
    treebank_model: Arc<Mutex<StatefulTreebankModel>>,
) {
    let dep_widget = create_dependency_tree_widget(&mut treebank_model.lock().unwrap());

    let scroll = gtk::ScrolledWindow::new(None, None);
    scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    scroll.add(dep_widget.borrow().inner());

    let sent_widget = create_sentence_widget(&mut treebank_model.lock().unwrap());

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    vbox.pack_start(&scroll, true, true, 0);
    vbox.pack_start(sent_widget.borrow().inner(), false, false, 0);

    let window = gtk::ApplicationWindow::new(application);
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

    treebank_model.lock().unwrap().first();
}

thread_local!(
    static DEPTREE_KEY: RefCell<Option<(Rc<RefCell<DependencyTreeWidget>>, Receiver<DependencyGraph>)>> = RefCell::new(None)
);

fn create_dependency_tree_widget(
    treebank_model: &mut StatefulTreebankModel,
) -> Rc<RefCell<DependencyTreeWidget>> {
    let dep_widget = Rc::new(RefCell::new(DependencyTreeWidget::new()));

    let (tx, rx) = channel();

    DEPTREE_KEY.with(clone!(dep_widget => move |global| {
        *global.borrow_mut() = Some((dep_widget, rx));
    }));

    // Notify widget when another tree is selected.
    treebank_model.connect_update(move |model| {
        let graph = ok_or!(model.graph(), return);
        tx.send(graph.clone())
            .expect("Could not send data to channel");
        glib::idle_add(|| {
            DEPTREE_KEY.with(|key| {
                if let Some((ref widget, ref rx)) = *key.borrow() {
                    if let Ok(graph) = rx.try_recv() {
                        if let Ok(svg) = graph.svg() {
                            if let Ok(handle) = Handle::new_from_data(svg.as_bytes()) {
                                widget.borrow_mut().update(handle);
                            }
                        }
                    }
                }
            });

            glib::Continue(false)
        });
    });

    dep_widget
}

thread_local!(
    static SENTENCE_KEY: RefCell<Option<(Rc<RefCell<SentenceWidget>>, Receiver<DependencyGraph>)>> = RefCell::new(None)
);

fn create_sentence_widget(
    treebank_model: &mut StatefulTreebankModel,
) -> Rc<RefCell<SentenceWidget>> {
    let sent_widget = Rc::new(RefCell::new(SentenceWidget::new()));

    let (tx, rx) = channel();

    SENTENCE_KEY.with(clone!(sent_widget => move |global| {
        *global.borrow_mut() = Some((sent_widget, rx));
    }));

    treebank_model.connect_update(move |model| {
        let graph = ok_or!(model.graph(), return);
        tx.send(graph.clone())
            .expect("Could not send data to channel");
        glib::idle_add(|| {
            SENTENCE_KEY.with(|key| {
                if let Some((ref widget, ref rx)) = *key.borrow() {
                    if let Ok(graph) = rx.try_recv() {
                        let tokens = graph.tokens();
                        widget.borrow_mut().update(tokens.join(" "));
                    }
                }
            });

            glib::Continue(false)
        });
    });

    sent_widget
}

fn setup_key_event_handling(
    window: &gtk::ApplicationWindow,
    treebank_model: Arc<Mutex<StatefulTreebankModel>>,
    dep_widget: Rc<RefCell<DependencyTreeWidget>>,
) {
    let window_clone = window.clone();

    window.connect_key_press_event(move |_, key_event| {
        println!("key: {}", key_event.get_keyval());
        match key_event.get_keyval() {
            DOT_KEY => match save_dot(&treebank_model.lock().unwrap()) {
                Ok(filename) => println!("Saved tree to: {}", filename),
                Err(err) => eprintln!("Error writing dot output: {}", err),
            },
            NEXT_KEY => {
                treebank_model.lock().unwrap().next();
            }
            PREVIOUS_KEY => {
                treebank_model.lock().unwrap().previous();
            }
            QUIT_KEY => {
                window_clone.destroy();
            }
            TIKZ_KEY => match save_tikz(&treebank_model.lock().unwrap()) {
                Ok(filename) => println!("Saved tree to: {}", filename),
                Err(err) => eprintln!("Error writing dot output: {}", err),
            },
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

fn save_dot(treebank_model: &StatefulTreebankModel) -> Result<String> {
    let graph = match treebank_model.graph() {
        Some(graph) => graph,
        None => return Err(ErrorKind::NoGraphSelected.into()),
    };

    let filename = format!("s{}.dot", treebank_model.idx());
    let mut writer = BufWriter::new(File::create(&filename)?);

    let dot = graph.dot()?;
    writer.write_all(dot.as_bytes())?;

    Ok(filename)
}

fn save_tikz(treebank_model: &StatefulTreebankModel) -> Result<String> {
    let graph = match treebank_model.graph() {
        Some(graph) => graph,
        None => return Err(ErrorKind::NoGraphSelected.into()),
    };

    let filename = format!("s{}.tikz", treebank_model.idx());
    let mut writer = BufWriter::new(File::create(&filename)?);

    let tikz = graph.tikz()?;
    writer.write_all(tikz.as_bytes())?;

    Ok(filename)
}
