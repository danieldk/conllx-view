use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{DrawingArea, TextView};
use rsvg::{Handle, HandleExt};

use model::TreebankModel;

pub struct DependencyTreeWidget {
    drawing_area: DrawingArea,
    scale: Rc<RefCell<Option<f64>>>,
    treebank_model: Rc<RefCell<TreebankModel>>,
    treebank_idx: Rc<RefCell<usize>>,
}

impl Deref for DependencyTreeWidget {
    type Target = DrawingArea;

    fn deref(&self) -> &DrawingArea {
        &self.drawing_area
    }
}

impl DependencyTreeWidget {
    pub fn new(treebank_model: Rc<RefCell<TreebankModel>>) -> Self {
        let mut widget = DependencyTreeWidget {
            drawing_area: DrawingArea::new(),
            scale: Rc::new(RefCell::new(None)),
            treebank_model,
            treebank_idx: Rc::new(RefCell::new(0)),
        };

        widget.setup_drawing_area();

        widget
    }

    pub fn inner(&self) -> &DrawingArea {
        &self.drawing_area
    }

    pub fn next(&mut self) {
        if *self.treebank_idx.borrow() == self.treebank_model.borrow().len() - 1 {
            return;
        }

        *self.treebank_idx.borrow_mut() += 1;
        *self.scale.borrow_mut() = None;

        self.drawing_area.queue_draw();
    }

    pub fn previous(&mut self) {
        if *self.treebank_idx.borrow() == 0 {
            return;
        }

        *self.treebank_idx.borrow_mut() -= 1;
        *self.scale.borrow_mut() = None;

        self.drawing_area.queue_draw();
    }

    fn setup_drawing_area(&mut self) {
        let treebank_idx = self.treebank_idx.clone();
        let treebank_model = self.treebank_model.clone();
        let scale = self.scale.clone();

        self.drawing_area.connect_draw(move |drawing_area, cr| {
            let handle = treebank_model
                .borrow()
                .handle(*treebank_idx.borrow())
                .expect("Could not retrieve tree");

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

pub struct SentenceWidget {
    text_view: TextView,
    treebank_model: Rc<RefCell<TreebankModel>>,
    treebank_idx: Rc<RefCell<usize>>,
}

impl Deref for SentenceWidget {
    type Target = TextView;

    fn deref(&self) -> &Self::Target {
        &self.text_view
    }
}

impl SentenceWidget {
    pub fn new(treebank_model: Rc<RefCell<TreebankModel>>) -> Self {
        let text_view = TextView::new();
        text_view.set_editable(false);
        text_view
            .get_buffer()
            .unwrap()
            .set_text(&format!("Sentence {}", 0));

        SentenceWidget {
            text_view,
            treebank_model,
            treebank_idx: Rc::new(RefCell::new(0)),
        }
    }

    pub fn inner(&self) -> &TextView {
        &self.text_view
    }

    pub fn next(&mut self) {
        if *self.treebank_idx.borrow() == self.treebank_model.borrow().len() - 1 {
            return;
        }

        *self.treebank_idx.borrow_mut() += 1;

        self.text_view
            .get_buffer()
            .unwrap()
            .set_text(&format!("Sentence {}", *self.treebank_idx.borrow_mut()));
    }

    pub fn previous(&mut self) {
        if *self.treebank_idx.borrow() == 0 {
            return;
        }

        *self.treebank_idx.borrow_mut() -= 1;

        self.text_view
            .get_buffer()
            .unwrap()
            .set_text(&format!("Sentence {}", *self.treebank_idx.borrow_mut()));
    }
}
