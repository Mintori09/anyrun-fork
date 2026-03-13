use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct MagikaOutput {
    pub result: MagikaResult,
}

#[derive(Deserialize, Debug)]
pub struct MagikaResult {
    pub value: MagikaValue,
}

#[derive(Deserialize, Debug)]
pub struct MagikaValue {
    pub output: MagikaDetails,
    pub score: f64,
}

#[derive(Deserialize, Debug)]
pub struct MagikaDetails {
    pub label: String,
    pub group: String,
}
