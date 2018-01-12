#[derive(Debug, Fail)]
pub enum ViewerError {
    #[fail(display = "no graph is selected")] NoGraphSelected,
}
