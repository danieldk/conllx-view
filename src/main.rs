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
use std::ops::Deref;
use std::process::{self, Command, Stdio};
use std::rc::Rc;

use dot::render;
use getopts::Options;
use gtk::prelude::*;
use gtk::{DrawingArea, PolicyType, Viewport};
use rsvg::{Handle, HandleExt};
use stdinout::{Input, OrExit};

mod graph;
use graph::{sentence_to_graph, DependencyGraph};

const ZOOM_IN_KEY: u32 = 61;
const ZOOM_OUT_KEY: u32 = 45;

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

struct DependencyTreeWidget {
    drawing_area: DrawingArea,
    handle: Rc<RefCell<Option<Handle>>>,
    scale: Rc<RefCell<Option<f64>>>,
}

impl Deref for DependencyTreeWidget {
    type Target = DrawingArea;

    fn deref(&self) -> &DrawingArea {
        &self.drawing_area
    }
}

impl DependencyTreeWidget {
    pub fn new() -> Self {
        DependencyTreeWidget {
            drawing_area: DrawingArea::new(),
            handle: Rc::new(RefCell::new(None)),
            scale: Rc::new(RefCell::new(None)),
        }
    }

    pub fn inner(&self) -> &DrawingArea {
        &self.drawing_area
    }

    pub fn set_graph(&mut self, graph: &DependencyGraph) {
        let mut dot = Vec::new();
        render(graph, &mut dot).or_exit("Error writing dot output", 1);
        let svg = dot_to_svg(&dot);

        let handle_clone = self.handle.clone();
        *handle_clone.borrow_mut() =
            Some(Handle::new_from_data(svg.as_bytes()).or_exit("Error parsing SVG", 1));

        let scale_clone = self.scale.clone();
        *scale_clone.borrow_mut() = None;

        self.drawing_area.connect_draw(move |drawing_area, cr| {
            let handle = handle_clone.borrow();

            if handle.is_none() {
                return Inhibit(false);
            }

            let handle = handle.as_ref().unwrap();

            let mut scale = scale_clone.borrow_mut();
            let svg_dims = handle.get_dimensions();
            if scale.is_none() {
                let da_width = drawing_area.get_allocated_width();
                let da_height = drawing_area.get_allocated_height();

                let scale_x = da_width as f64 / svg_dims.width as f64;
                let scale_y = da_height as f64 / svg_dims.height as f64;

                *scale = Some(scale_x.min(scale_y));
            }

            let scale = scale.unwrap();

            drawing_area.set_size_request(
                (svg_dims.width as f64 * scale).ceil() as i32,
                (svg_dims.height as f64 * scale).ceil() as i32,
            );

            cr.scale(scale, scale);
            cr.paint_with_alpha(0.0);
            handle.render_cairo(&cr);

            Inhibit(false)
        });
    }

    pub fn zoom_in(&mut self) {
        let mut opt_scale = self.scale.borrow_mut();
        *opt_scale = opt_scale.map(|scale| scale / 0.90);
    }

    pub fn zoom_out(&mut self) {
        let mut opt_scale = self.scale.borrow_mut();
        *opt_scale = opt_scale.map(|scale| scale * 0.90);
    }
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
    dep_widget.borrow_mut().set_graph(graph);
    let dep_widget_clone = dep_widget.clone();

    window.connect_key_press_event(move |_, key_event| {
        println!("key: {}", key_event.get_keyval());
        match key_event.get_keyval() {
            ZOOM_IN_KEY => {
                let mut widget_mut = dep_widget_clone.borrow_mut();
                widget_mut.zoom_in();
                widget_mut.queue_draw();
            },
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
