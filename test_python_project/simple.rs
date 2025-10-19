// Simple Rust test file

pub struct User {
    pub id: u32,
    pub name: String,
}

impl User {
    pub fn new(id: u32, name: String) -> Self {
        Self { id, name }
    }

    pub fn display(&self) -> String {
        format!("User {}: {}", self.id, self.name)
    }
}

pub fn create_user(id: u32, name: String) -> User {
    User::new(id, name)
}

pub fn main() {
    let user = create_user(1, "Alice".to_string());
    println!("{}", user.display());
}