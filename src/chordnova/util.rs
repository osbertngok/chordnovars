use std::fmt;

pub fn iterable_to_str<I, D>(iterable: I) -> String
    where
        I: IntoIterator<Item = D>,
        D: fmt::Display,
{
    let mut iterator = iterable.into_iter();

    let head = match iterator.next() {
        None => return String::from("[]"),
        Some(x) => format!("[{}", x),
    };
    let body = iterator.fold(head, |a, v| format!("{}, {}", a, v));
    format!("{}]", body)
}