use std::cell::RefCell;
use std::mem;
use std::ops::Deref;
use std::rc::Rc;

use glib::SignalHandlerId;
use gtk::prelude::*;
use gtk::DrawingArea;
use rsvg::{Handle, HandleExt};

use error::Result;
use model::TreebankModel;

pub struct DependencyTreeWidget {
    drawing_area: DrawingArea,
    draw_handler: Option<SignalHandlerId>,
    scale: Rc<RefCell<Option<f64>>>,
    treebank_model: TreebankModel,
    treebank_idx: usize,
}

impl Deref for DependencyTreeWidget {
    type Target = DrawingArea;

    fn deref(&self) -> &DrawingArea {
        &self.drawing_area
    }
}

impl DependencyTreeWidget {
    pub fn new(treebank_model: TreebankModel) -> Self {
        DependencyTreeWidget {
            drawing_area: DrawingArea::new(),
            draw_handler: None,
            scale: Rc::new(RefCell::new(None)),
            treebank_model,
            treebank_idx: 0,
        }
    }

    pub fn inner(&self) -> &DrawingArea {
        &self.drawing_area
    }

    pub fn next(&mut self) -> Result<()> {
        if self.treebank_idx == self.treebank_model.len() - 1 {
            return Ok(());
        }

        self.treebank_idx += 1;
        *self.scale.borrow_mut() = None;

        self.show_graph()
    }

    pub fn previous(&mut self) -> Result<()> {
        if self.treebank_idx == 0 {
            return Ok(());
        }

        self.treebank_idx -= 1;
        *self.scale.borrow_mut() = None;

        self.show_graph()
    }

    pub fn show_graph(&mut self) -> Result<()> {
        // FIXME: what to do with an empty treebank?
        assert!(
            self.treebank_idx < self.treebank_model.len(),
            "Widget has invalid treebank index"
        );

        let handle = self.treebank_model.handle(self.treebank_idx)?;

        let scale = self.scale.clone();
        *scale.borrow_mut() = None;

        let mut draw_handler = None;
        mem::swap(&mut draw_handler, &mut self.draw_handler);
        if let Some(draw_handler) = draw_handler {
            self.drawing_area.disconnect(draw_handler);
        }

        self.draw_handler = Some(self.drawing_area.connect_draw(move |drawing_area, cr| {
            // White canvas.
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.paint();

            cr.save();

            // Translate to center SVG.
            let (x_offset, y_offset) = compute_centering_offset(drawing_area, &handle);
            cr.translate(x_offset, y_offset);

            // Scale the surface.
            let scale = *scale
                .borrow_mut()
                .get_or_insert(compute_scale(drawing_area, &handle));
            cr.scale(scale, scale);

            // Paint the SVG.
            cr.paint_with_alpha(0.0);
            handle.render_cairo(&cr);

            cr.restore();

            // Set size request, this is required for computing the scroll bars.
            let svg_dims = handle.get_dimensions();
            drawing_area.set_size_request(
                (svg_dims.width as f64 * scale).ceil() as i32,
                (svg_dims.height as f64 * scale).ceil() as i32,
            );

            Inhibit(false)
        }));

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
    let rect = drawing_area.get_allocation();

    let scale_x = rect.width as f64 / svg_dims.width as f64;
    let scale_y = rect.height as f64 / svg_dims.height as f64;

    scale_x.min(scale_y)
}

/// Computes the offset/translation for centering the SVG in the drawing area.
fn compute_centering_offset(drawing_area: &DrawingArea, handle: &Handle) -> (f64, f64) {
    let svg_dims = handle.get_dimensions();
    let scale = compute_scale(drawing_area, handle);

    let rect = drawing_area.get_allocation();

    (
        rect.width as f64 * 0.5 - svg_dims.width as f64 * scale * 0.5,
        rect.height as f64 * 0.5 - svg_dims.height as f64 * scale * 0.5,
    )
}
