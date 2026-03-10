use crate::shared::zellij;

pub fn run() -> Result<(), zellij::Error> {
    zellij::write_signal("question")
}
