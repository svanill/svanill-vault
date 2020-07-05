use crate::db::models::User;
use crate::errors::VaultError;
use diesel::prelude::*;

pub fn find_user_by_username(
    conn: &SqliteConnection,
    username: &str,
) -> Result<Option<User>, VaultError> {
    use crate::db::schema::user;

    let user = user::table
        .filter(user::username.eq(username))
        .first::<User>(conn)
        .optional()?;

    Ok(user)
}
