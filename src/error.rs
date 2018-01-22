use std::fmt;
use std::io;

use gtk;

error_chain! {
    errors {
        NoGraphSelected {
            description("no graph is selected")
        }
    }
    foreign_links {
        Fmt(fmt::Error);
        Io(io::Error);
        Gtk(gtk::Error);
    }
}
