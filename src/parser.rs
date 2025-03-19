use serde::Deserialize;
use anyhow::Result;
use models::*;
use serde_xml_rs::from_reader;
use std::fs::File;
use regex::Regex;

#[derive(Debug, Deserialize)]
pub struct XBRL {
    #[serde(rename = "context")]
    pub contexts: Vec<Context>,
    #[serde(rename = "unit")]
    pub units: Vec<Unit>,
    #[serde(rename = "numericItem")]
    pub numeric_items: Vec<NumericItem>,
}

#[derive(Debug, Deserialize)]
pub struct Context {
    #[serde(rename = "@id")]
    pub id: String,
    pub entity: String,
    pub period: Period,
}

#[derive(Debug, Deserialize)]
pub struct Period {
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub instant: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Unit {
    #[serde(rename = "@id")]
    pub id: String,
    pub measure: String,
}

#[derive(Debug, Deserialize)]
pub struct NumericItem {
    #[serde(rename = "@contextRef")]
    pub context_ref: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@unitRef")]
    pub unit_ref: String,
    #[serde(rename = "$value")]
    pub value: String,
}

/// Extract a numeric metric from XBRL data
fn extract_metric(xbrl: &XBRL, metric_name: &str, context_id: &str) -> Result<f64> {
    let item = xbrl.numeric_items
        .iter()
        .find(|item| item.name == metric_name && item.context_ref == context_id)
        .ok_or_else(|| anyhow::anyhow!("Metric not found"))?;

    // Clean numeric value (remove commas)
    let re = Regex::new(r"[^\d.]")?;
    let value = re.replace_all(&item.value, "").parse::<f64>()?;

    Ok(value)
}

fn read_xrbl(file: XBRL) -> Result<XBRL> {
    let file = File::open("TSLA_10K.txt")?;
    let xbrl: XBRL = from_reader(file)?;
    Ok(xbrl)

}

// example usage
// fn main() -> Result<()> {
//     // Read XBRL file
//     let file = File::open("TSLA_10K.txt")?;
//     let xbrl: XBRL = from_reader(file)?;

//     // Extract key metrics
//     let revenue_2024 = extract_metric(
//         &xbrl,
//         "us-gaap:RevenueFromContractWithCustomerExcludingAssessedTax",
//         "c-1",
//     )?;

//     let revenue_2023 = extract_metric(
//         &xbrl,
//         "us-gaap:RevenueFromContractWithCustomerExcludingAssessedTax",
//         "c-28",
//     )?;

//     // Calculate growth rate
//     let growth_rate = (revenue_2024 - revenue_2023) / revenue_2023 * 100.0;

//     println!("Revenue Growth: {:.2}%", growth_rate);

//     Ok(())
// }

