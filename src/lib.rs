use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum Bencode {
    String(String),
}

impl Bencode {
    pub fn new(encoded_value: &str) -> Result<Self, BencodeError> {
        if encoded_value
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
        {
            let (number_string, string) = encoded_value
                .split_once(':')
                .ok_or(BencodeError::MissingColonDelimeter)?;
            let number = number_string
                .parse()
                .map_err(|_| BencodeError::InvalidNumber)?;
            let decoded_value = string.get(..number).ok_or(BencodeError::InvalidLength)?;
            return Ok(Self::String(decoded_value.to_string()));
        }

        unreachable!()
    }

    pub fn to_value(&self) -> Value {
        match self {
            Bencode::String(string) => Value::String(string.to_owned()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BencodeError {
    MissingColonDelimeter,
    InvalidNumber,
    InvalidLength,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_string() {
        assert_eq!(
            Bencode::new("5:hello"),
            Ok(Bencode::String("hello".to_string()))
        );
        assert_eq!(
            Bencode::new("5hello"),
            Err(BencodeError::MissingColonDelimeter)
        );
        assert_eq!(Bencode::new("5a:hello"), Err(BencodeError::InvalidNumber));
        assert_eq!(Bencode::new("6:hello"), Err(BencodeError::InvalidLength));
    }
}
