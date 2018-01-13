use std::io;

use gtk;

error_chain! {
    foreign_links {
        Io(io::Error);
        Gtk(gtk::Error);
    }
}
