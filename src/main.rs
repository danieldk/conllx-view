extern crate cairo;
extern crate conllx;
extern crate dot;
extern crate getopts;
extern crate gtk;
extern crate petgraph;
extern crate rsvg;
extern crate stdinout;

use std::cell::RefCell;
use std::env::args;
use std::io::{Read, Write};
use std::process::{self, Command, Stdio};
use std::rc::Rc;

use cairo::Context;
use dot::render;
use getopts::Options;
use gtk::prelude::*;
use gtk::{DrawingArea, PolicyType, Viewport};
use rsvg::{Handle, HandleExt};
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

    process
        .stdin
        .unwrap()
        .write_all(dot)
        .or_exit("Cannot write to dot stdin", 1);

    let mut svg = String::new();
    process
        .stdout
        .unwrap()
        .read_to_string(&mut svg)
        .or_exit("Cannot read dot stdout", 1);

    svg
}

struct SVGTree {
    handle: Handle,
    scale: Option<f64>,
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

    let mut dot = Vec::new();
    render(&graph, &mut dot).or_exit("Error writing dot output", 1);
    let svg = dot_to_svg(&dot);

    gtk::init().or_exit("Failed to initialize GTK", 1);

    // FIXME: should not terminate the viewer.
    let svg_tree = Rc::new(RefCell::new(SVGTree{handle: Handle::new_from_data(svg.as_bytes()).or_exit("Error parsing SVG", 1), scale: None}));
    let svg_tree_clone = svg_tree.clone();

    // SVG drawing from rsvg-rs example.
    drawable(800, 600, move |drawing_area, cr| {
        let mut svg_tree = svg_tree_clone.borrow_mut();
        let svg_dims = svg_tree.handle.get_dimensions();

        if svg_tree.scale.is_none() {
            let da_width = drawing_area.get_allocated_width();
            let da_height = drawing_area.get_allocated_height();

            let scale_x = da_width as f64 / svg_dims.width as f64;
            let scale_y = da_height as f64 / svg_dims.height as f64;

            svg_tree.scale = Some(scale_x.min(scale_y));
        }

        let scale = svg_tree.scale.unwrap();

        cr.scale(scale, scale);

        drawing_area.set_size_request((svg_dims.width as f64 * scale).ceil() as i32, (svg_dims.height as f64 * scale).ceil() as i32);

        cr.paint_with_alpha(0.0);
        svg_tree.handle.render_cairo(&cr);

        Inhibit(false)
    });

    gtk::main();
}

pub fn drawable<F>(width: i32, height: i32, draw_fn: F)
where
    F: Fn(&DrawingArea, &Context) -> Inhibit + 'static,
{
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("conllx-view");
    window.set_border_width(10);

    let drawing_area = Box::new(DrawingArea::new)();
    drawing_area.set_size_request(100, 100);
    drawing_area.connect_draw(draw_fn);

    let viewport = Viewport::new(None, None);
    viewport.add(&drawing_area);

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
