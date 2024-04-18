pub mod settings;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use settings::{Address, AffaldVarmeConfig};
use url::Url;

use self::settings::{AddressId, TraditionalAddress};

pub async fn get_containers(config: AffaldVarmeConfig) -> Result<Vec<Container>, String> {
    let response = fetch_remote_response(config)
        .await
        .map_err(|err| format!("Error connecting: {:?}", err))?;

    if !response.status().is_success() {
        return Err(format!("Unexpected status code: {:?}", response.status()));
    }

    response
        .json::<Respon>()
        .await
        .map_err(|err| format!("Error reading response content: {:?}", err))
        .and_then(|respon| {
            respon
                .0
                .into_iter()
                .next()
                .ok_or_else(|| "No data found".to_string())
                .map(|response| {
                    println!("Received information for stand: {}", response.stand_name);
                    response.into()
                })
        })
}

#[derive(Deserialize)]
struct Respon(Vec<Response>);

#[derive(Deserialize)]
struct Response {
    #[allow(dead_code)]
    #[serde(rename = "standId")]
    stand_id: String,

    #[serde(rename = "standName")]
    stand_name: String,

    #[serde(rename = "plannedLoads")]
    planned_loads: Vec<PlannedLoad>,
}

#[derive(Deserialize)]
struct PlannedLoad {
    date: DateTime<Utc>,
    fractions: Vec<String>,
}

async fn fetch_remote_response(
    config: AffaldVarmeConfig,
) -> Result<reqwest::Response, reqwest::Error> {
    let remote_url = build_remote_url(config).await;

    reqwest::get(remote_url).await
}

async fn build_remote_url(config: AffaldVarmeConfig) -> Url {
    let mut url_builder = config.base_url.clone();

    let address_id = match config.address {
        Address::Id(x) => x.id.clone(),
        Address::FullySpecified(x) => lookup_address(x).await.unwrap().id.clone(),
    };

    url_builder.set_path(format!("api/calendar/address/{}", dbg!(address_id)).as_str());

    url_builder
}

async fn lookup_address(address: TraditionalAddress) -> Result<AddressId, String> {
    let mut url_builder = address.address_lookup_url.clone();
    url_builder.set_path("adresser");

    url_builder
        .query_pairs_mut()
        .append_pair(
            "q",
            format!(
                "{} {} {} {}",
                address.street_name, address.street_no, address.postal_code, address.city
            )
            .as_str(),
        )
        .append_pair("per_side", "2");

    let response = reqwest::get(url_builder)
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    response
        .as_array()
        .and_then(|x| if x.len() == 1 { x.first() } else { None })
        .and_then(|x| {
            x.get("kvhx")
                .and_then(|x| x.as_str())
                .map(|x| AddressId { id: x.to_owned() })
        })
        .ok_or_else(|| "Address not found".to_string())
}

#[derive(Debug, PartialEq)]
pub struct Container {
    pub name: String,
    pub date: DateTime<Utc>,
}

impl Container {
    fn new(name: String, date: DateTime<Utc>) -> Self {
        Self { name, date }
    }
}

impl From<Response> for Vec<Container> {
    fn from(response: Response) -> Self {
        response
            .planned_loads
            .into_iter()
            .flat_map(move |x| {
                x.fractions
                    .into_iter()
                    .map(move |y| Container::new(y, x.date))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mitaffald::settings::{Address, AddressId, TraditionalAddress};
    use fluent_asserter::{prelude::StrAssertions, *};
    use mockito::Matcher;

    #[tokio::test]
    async fn can_extract_data_using_address_id() {
        let mut remote = mockito::Server::new_async().await;
        let address_id = "123".to_string();
        let config = AffaldVarmeConfig {
            address: Address::Id(AddressId {
                id: address_id.clone(),
            }),
            base_url: Url::parse(remote.url().as_str()).unwrap(),
        };

        let remote = remote
            .mock(
                "GET",
                format!("/api/calendar/address/{}", address_id).as_str(),
            )
            .with_status(200)
            .with_body_from_file("src/mitaffald/remote_responses/container_information.json")
            .create_async()
            .await;

        let actual = get_containers(config).await;

        remote.assert_async().await;
        assert_that!(actual.is_ok()).is_true();
        insta::assert_debug_snapshot!(actual.unwrap());
    }

    #[tokio::test]
    async fn can_extract_data_using_traditional_address() {
        let mut remote = mockito::Server::new_async().await;
        let config = AffaldVarmeConfig {
            address: Address::FullySpecified(TraditionalAddress {
                street_name: "Kongevejen".to_string(),
                street_no: "100".to_string(),
                postal_code: "8000".to_string(),
                city: "Aarhus C".to_string(),
                address_lookup_url: Url::parse(remote.url().as_str())
                    .expect("Failed to parse address_lookup_url"),
            }),
            base_url: Url::parse(remote.url().as_str()).expect("Failed to parse base_url"),
        };

        let address_lookup_mock = remote
            .mock("GET", "/adresser")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("q".into(), "Kongevejen 100 8000 Aarhus C".into()),
                Matcher::UrlEncoded("per_side".into(), "2".into()),
            ]))
            .with_status(200)
            .with_body_from_file("src/mitaffald/remote_responses/address_lookup.json")
            .create_async()
            .await;

        let container_info_mock = remote
            .mock("GET", "/api/calendar/address/07514448_100_______")
            .with_status(200)
            .with_body_from_file("src/mitaffald/remote_responses/container_information.json")
            .create_async()
            .await;

        let actual = get_containers(config).await;

        address_lookup_mock.assert_async().await;
        container_info_mock.assert_async().await;
        assert_that!(actual.is_ok()).is_true();
        insta::assert_debug_snapshot!(actual.unwrap());
    }

    #[tokio::test]
    async fn can_handle_server_error() {
        let mut remote = mockito::Server::new_async().await;
        let config = AffaldVarmeConfig {
            address: Address::Id(AddressId { id: "123".into() }),
            base_url: Url::parse(remote.url().as_str()).unwrap(),
        };

        let remote = remote
            .mock("GET", mockito::Matcher::Regex(".*".to_string()))
            .with_status(500)
            .create_async()
            .await;

        let actual = get_containers(config).await;

        remote.assert_async().await;
        assert_that!(actual.is_err()).is_true();
        assert_that!(actual.unwrap_err()).contains("Unexpected status code");
    }

    #[tokio::test]
    async fn can_handle_no_responses() {
        let config = AffaldVarmeConfig {
            address: Address::Id(AddressId { id: "123".into() }),
            base_url: Url::parse("http://127.0.0.1:12312").unwrap(),
        };

        let actual = get_containers(config).await;

        assert_that!(actual.is_err()).is_true();
        assert_that!(actual.unwrap_err()).contains("Error connecting");
    }
}
