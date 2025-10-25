use eure_derive::Eure;
use eure_schema::{ToEureSchema, Type};
use serde::{Deserialize, Serialize};

#[derive(Eure, Serialize, Deserialize)]
enum Status {
    Success { message: String },
    Error { code: u32, message: String },
    Pending,
}

fn main() {
    let schema = Status::eure_schema();
    println!("Status schema: {:#?}", schema);

    if let Type::Variants(variant_schema) = &schema.type_expr {
        println!(
            "\nVariants: {:?}",
            variant_schema.variants.keys().collect::<Vec<_>>()
        );
        println!("Representation: {:?}", variant_schema.representation);
    }
}
