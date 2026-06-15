#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomHand(pub Vec<u32>);

impl TryFrom<String> for CustomHand {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut parsed_values = Vec::new();

        for code in value.split(",") {
            let parsed = code.parse::<u32>()?;
            parsed_values.push(parsed);
        }

        Ok(CustomHand(parsed_values))
    }
}
