use std::any::type_name;

pub fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}