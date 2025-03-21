use std::fmt::Display;

use validator::ValidateEmail;

#[derive(Debug, Clone)]
pub struct SubscriberEmail(String);

impl Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    } 
    
}

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if s.validate_email() {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid subscriber email address.", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use quickcheck::{Arbitrary, Gen};
    
    #[derive(Debug, Clone)]
    struct ValidEmail(String);

    impl Arbitrary for ValidEmail {
        fn arbitrary(_g: &mut Gen) -> Self {
            let email = SafeEmail().fake();
            ValidEmail(email)
        } 
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email_is_accepted(valid_email: ValidEmail) -> bool {
        dbg!(&valid_email.0);
        SubscriberEmail::parse(valid_email.0).is_ok()
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursula.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

}