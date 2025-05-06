use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        let is_empty_or_whitespace = s.trim().is_empty();

        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|char| forbidden_characters.contains(&char));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(format!("{} is not a valid subscriber name.", s))
        } else {
            Ok(SubscriberName(s))
        }
    } 
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_256_grapheme_name_is_valid() {
        let name = "中".repeat(256);
        claim::assert_ok!(SubscriberName::parse(name));
    }
    
    #[test]
    fn a_257_grapheme_name_is_rejected() {
        let name = "中".repeat(257);
        claim::assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_name_is_rejected() {
        let name = " ".to_string();
        claim::assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_name_is_rejected() {
        let name = "".to_string();
        claim::assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn name_with_forbidden_characters_is_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            claim::assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        claim::assert_ok!(SubscriberName::parse(name));
    }
}