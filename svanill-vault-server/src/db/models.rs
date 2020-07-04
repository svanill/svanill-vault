#[derive(Queryable)]
pub struct User {
    pub username: String,
    pub challenge: String,
    pub answer: String,
}
