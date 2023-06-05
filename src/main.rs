use crate::catalogue::Catalogue;

mod catalogue;
mod day;
mod week;

fn main() {
    let mut catalogue = Catalogue::new();
    for pdf in [include_bytes!("../S12.pdf").as_slice(), include_bytes!("../S19-2023.pdf").as_slice(), include_bytes!("../S20-2023.pdf").as_slice(), include_bytes!("../S23-2023.pdf").as_slice()] {
        let days = week::parse_pdf(pdf).unwrap();
        dbg!(catalogue.insert(days));
    }

    // dbg!(&catalogue);
}

