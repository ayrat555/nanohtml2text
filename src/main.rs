use nanohtml2text::html2text;
use std::io::Read;

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();
    println!("{}", html2text(&buffer));
}
