mod day;
mod week;

fn main() {
    let pdf = include_bytes!("../S23-2023.pdf");
    let days = week::parse_pdf(pdf);

    dbg!(&days);
}

