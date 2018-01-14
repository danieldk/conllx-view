use std::cell::RefCell;
use std::io::{Read, Write};
use std::ops::Deref;
use std::process::{Command, Stdio};
use std::rc::Rc;

use dot::render;
use gtk::prelude::*;
use gtk::DrawingArea;
use rsvg::{Handle, HandleExt};

use error::Result;
use graph::DependencyGraph;

fn dot_to_svg(dot: &[u8]) -> Result<String> {
    // FIXME: bind against C library?

    // Spawn Graphviz dot for rendering SVG (Fixme: bind against C library?).
    let process = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    process.stdin.unwrap().write_all(dot)?;

    let mut svg = String::new();
    process.stdout.unwrap().read_to_string(&mut svg)?;

    Ok(svg)
}

pub struct DependencyTreeWidget {
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

    pub fn set_graph(&mut self, graph: &DependencyGraph) -> Result<()> {
        let mut dot = Vec::new();
        render(graph, &mut dot)?;
        let svg = dot_to_svg(&dot)?;

        let handle_clone = self.handle.clone();
        *handle_clone.borrow_mut() = Some(Handle::new_from_data(svg.as_bytes())?);

        let scale_clone = self.scale.clone();
        *scale_clone.borrow_mut() = None;

        self.drawing_area.connect_draw(move |drawing_area, cr| {
            let handle = handle_clone.borrow();

            if handle.is_none() {
                return Inhibit(false);
            }

            let handle = handle.as_ref().unwrap();

            // Translate to center SVG.
            let (x_offset, y_offset) = compute_centering_offset(drawing_area, handle);
            cr.translate(x_offset, y_offset);

            // Scale the surface.
            let scale = *scale_clone
                .borrow_mut()
                .get_or_insert(compute_scale(drawing_area, handle));
            cr.scale(scale, scale);

            // Paint the SVG.
            cr.paint_with_alpha(0.0);
            handle.render_cairo(&cr);

            // Set size request, this is required for computing the scroll bars.
            let svg_dims = handle.get_dimensions();
            drawing_area.set_size_request(
                (svg_dims.width as f64 * scale).ceil() as i32,
                (svg_dims.height as f64 * scale).ceil() as i32,
            );

            Inhibit(false)
        });

        Ok(())
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

pub fn compute_scale(drawing_area: &DrawingArea, handle: &Handle) -> f64 {
    let svg_dims = handle.get_dimensions();

    let da_width = drawing_area.get_allocated_width();
    let da_height = drawing_area.get_allocated_height();

    let scale_x = da_width as f64 / svg_dims.width as f64;
    let scale_y = da_height as f64 / svg_dims.height as f64;

    scale_x.min(scale_y)
}

/// Computes the offset/translation for centering the SVG in the drawing area.
fn compute_centering_offset(drawing_area: &DrawingArea, handle: &Handle) -> (f64, f64) {
    let svg_dims = handle.get_dimensions();
    let scale = compute_scale(drawing_area, handle);

    let da_width = drawing_area.get_allocated_width();
    let da_height = drawing_area.get_allocated_height();

    (
        da_width as f64 * 0.5 - svg_dims.width as f64 * scale * 0.5,
        da_height as f64 * 0.5 - svg_dims.height as f64 * scale * 0.5,
    )
}
