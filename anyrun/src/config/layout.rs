use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, ValueEnum)]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

#[derive(Deserialize, Serialize, Clone, Debug, ValueEnum)]
pub enum KeyboardMode {
    Exclusive,
    OnDemand,
}

// Could have a better name
#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum RelativeNum {
    Absolute(i32),
    Fraction(f64),
}

impl RelativeNum {
    pub fn to_val(&self, val: u32) -> i32 {
        match self {
            RelativeNum::Absolute(num) => *num,
            RelativeNum::Fraction(frac) => (frac * val as f64) as i32,
        }
    }
}

impl From<&str> for RelativeNum {
    fn from(value: &str) -> Self {
        let (ty, val) = value.split_once(':').expect("Invalid RelativeNum value");

        match ty {
            "absolute" => Self::Absolute(val.parse().unwrap()),
            "fraction" => Self::Fraction(val.parse().unwrap()),
            _ => panic!("Invalid type of value"),
        }
    }
}

#[derive(Deserialize, Clone, ValueEnum)]
pub enum Position {
    Top,
    Center,
}
