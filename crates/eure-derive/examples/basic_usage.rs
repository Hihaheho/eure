use eure_derive::Eure;
use eure_schema::ToEureSchema;
use serde::{Deserialize, Serialize};

#[derive(Eure, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct User {
    user_id: String,
    display_name: String,
    #[serde(rename = "email_address")]
    email: Option<String>,
    age: u32,
    tags: Vec<String>,
}

#[derive(Eure, Serialize, Deserialize, Debug)]
enum UserStatus {
    Active {
        since: String,
    },
    Inactive,
    Banned {
        reason: String,
        until: Option<String>,
    },
}

fn main() {
    // Get the schema for User
    let user_schema = User::eure_schema();
    println!("User schema:");
    println!("{user_schema:#?}");
    println!("\nType name: {:?}", User::type_name());

    // Get the schema for UserStatus
    let status_schema = UserStatus::eure_schema();
    println!("\nUserStatus schema:");
    println!("{status_schema:#?}");
    println!("\nType name: {:?}", UserStatus::type_name());

    // You can now use these schemas for validation, documentation,
    // or converting to other schema formats like JSON Schema
}
