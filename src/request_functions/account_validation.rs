pub fn check_passwords(password_one: &str, password_two: &str) -> Option<&'static str> {
    if password_one == password_two {
        if password_one.len() <= 128 && password_one.len() >= 8 {
            None
        } else {
            Some("Please use a password between 8 and 128 characters long.")
        }
    } else {
        Some("Your passwords do not match. Please try again.")
    }
}
pub fn check_email(email: &str) -> Option<&str> {
    if email.len() <= 64 {
        if let None = super::get_like("users", "email", email).get(0) {
            None
        } else {
            Some("Sorry, your email address has already been registered. Please use a different email address or log in with your account.")
        }
    } else {
        Some("Sorry, your email address is too long. Please use a different email address.")
    }
}
pub fn check_username(username: &str) -> Option<&'static str> {
    if username.len() <= 32 && username.len() >= 4 {
        if let None = super::get_like("users", "username", username).get(0) {
            None
        } else {
            Some("Sorry, this username has already been taken. Please select another.")
        }
    } else {
        Some("Sorry, your username is invalid. Please use between 4 and 32 characters for your username.")
    }
}
