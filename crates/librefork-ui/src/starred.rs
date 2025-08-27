#[derive(Hash, Eq, PartialEq, Clone)]
pub enum StarredItem {
    Branch(String),
    Commit(String),
}
