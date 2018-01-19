use std::fmt;
use std::io;

use gtk;

error_chain! {
    foreign_links {
        Fmt(fmt::Error);
        Io(io::Error);
        Gtk(gtk::Error);
    }
}
