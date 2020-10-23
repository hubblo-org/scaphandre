pub mod stdout;

pub trait Exporter {
    fn run (&mut self);
}