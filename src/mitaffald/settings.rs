use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AffaldVarmeConfig {
    pub address: Address,
    pub base_url: Url,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Address {
    Id(AddressId),
    FullySpecified(TraditionalAddress),
}

#[derive(Deserialize, Debug)]
pub struct TraditionalAddress {
    pub street_name: String,
    pub street_no: String,
    pub postal_code: String,
    pub city: String,
}

#[derive(Deserialize, Debug)]
pub struct AddressId {
    pub id: String,
}
