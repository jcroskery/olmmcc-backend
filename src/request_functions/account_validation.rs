pub fn check_password(password: &str) -> Option<&'static str> {
    if password.len() <= 128 && password.len() >= 8 {
        None
    } else {
        Some("Please use a password between 8 and 128 characters long.")
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

pub fn check_subscription(subscription: &str) -> Option<&str> {
    if let Ok(t) = subscription.parse::<i32>() {
        if t > -1 && t < 3 {
            return None;
        }
    }
    Some("Invalid subscription policy!")
}
