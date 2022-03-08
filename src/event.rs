use crate::PathItem;

#[derive(Debug)]
pub enum Event {
    SearchFoundPath(PathItem),
    SearchFinished,
}
