pub mod settings;

use chrono::{Datelike, Local, NaiveDate};
use easy_scraper::Pattern;
use settings::{Address, AffaldVarmeConfig};
use std::collections::BTreeMap;
use tracing::instrument;
use url::Url;

#[instrument]
pub async fn get_containers(config: AffaldVarmeConfig) -> Result<Vec<Container>, String> {
    let response = fetch_remote_response(config).await;

    if response.is_err() {
        return Err(format!("Error connecting: {:?}", response.err()));
    }

    let response = response.unwrap();
    if !response.status().is_success() {
        return Err(format!("Unexpected status code: {:?}", response.status()));
    }

    match response.text().await {
        Ok(text) => parse_response(text),
        Err(err_reading_text) => Err(format!(
            "Error reading response content: {:?}",
            err_reading_text
        )),
    }
}

async fn fetch_remote_response(
    config: AffaldVarmeConfig,
) -> Result<reqwest::Response, reqwest::Error> {
    let remote_url = build_remote_url(config);

    reqwest::get(remote_url).await
}

fn build_remote_url(config: AffaldVarmeConfig) -> Url {
    let mut url_builder = config.base_url.clone();
    url_builder.set_path("Adresse/VisAdresseInfo");

    match config.address {
        Address::Id(x) => {
            url_builder
                .query_pairs_mut()
                .append_pair("address-selected-id", x.id.as_str());
        }
        Address::FullySpecified(x) => {
            //The comma (URL encoded as `%2C`) after the street_name in the address-search query string is very important and prevents the website from finding the address if is missing.
            //https://mitaffald.affaldvarme.dk/Adresse/VisAdresseInfo?address-search=Kongevejen%2C+8000+Aarhus+C&number-search=100&address-selected-postnr=8000

            url_builder
                .query_pairs_mut()
                .append_pair(
                    "address-search",
                    format!("{}, {} {}", x.street_name, x.postal_code, x.city).as_str(),
                )
                .append_pair("number-search", x.street_no.as_str())
                .append_pair("address-selected-postnr", x.postal_code.as_str());
        }
    }

    url_builder
}

fn parse_response(html: String) -> Result<Vec<Container>, String> {
    match extract_error(html.as_str()) {
        None => Ok(extract_container_data(html.as_str())),
        Some(error_message) => Err(error_message),
    }
}

fn extract_error(html: &str) -> Option<String> {
    let pattern = Pattern::new(
        r#"
<div class="alert-warning">
    {{error}}
</div>
        "#,
    )
    .unwrap();

    let matches = pattern.matches(html);

    if !matches.is_empty() {
        return Some(matches[0].get("error").unwrap().clone());
    }

    None
}

fn extract_container_data(html: &str) -> Vec<Container> {
    let pattern = Pattern::new(
        r#"
    <h3>
        {{name}}
    </h3>
    <div>
    <table>
    <thead></thead>
    <tbody>
        <tr>
            <td> {{id}}</td>
            <td>{{size}}</td>
            <td>{{frequency}}</td>
            <td>{{next_empty}}</td>
        </tr>
    </tbody>
    </table>
    </div>
    "#,
    )
    .unwrap();

    pattern
        .matches(html)
        .into_iter()
        //.map(from_destructive)
        .map(from_nondestructive)
        .collect()
}

#[allow(dead_code)]
fn from_destructive(mut value: BTreeMap<String, String>) -> Container {
    Container {
        id: value.remove("id").unwrap_or_else(|| String::from("N/A")),
        name: value.remove("name").unwrap_or_else(|| String::from("N/A")),
        frequency: value
            .remove("frequency")
            .unwrap_or_else(|| String::from("N/A")),
        next_empty: value
            .remove("next_empty")
            .unwrap_or_else(|| String::from("N/A")),
        size: value.remove("size").unwrap_or_else(|| String::from("N/A")),
    }
}

fn from_nondestructive(value: BTreeMap<String, String>) -> Container {
    let default = String::from("N/A");

    Container {
        id: value.get("id").unwrap_or(&default).clone(),
        name: value.get("name").unwrap_or(&default).clone(),
        frequency: value.get("frequency").unwrap_or(&default).clone(),
        next_empty: value.get("next_empty").unwrap_or(&default).clone(),
        size: value.get("size").unwrap_or(&default).clone(),
    }
}

#[derive(Debug, PartialEq)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub frequency: String,
    pub next_empty: String,
    pub size: String,
}

impl Container {
    pub fn get_next_empty(&self) -> NaiveDate {
        /* next_empty is in the format DD/MM so we need to guess the year.
        Most of the times it will be current year, but if the date is in the past it will be next year.*/
        let mut parts = self.next_empty.split('/');

        let day = parts.next().unwrap();
        let month = parts.next().unwrap();

        let day = day.parse::<u32>().unwrap();
        let month = month.parse::<u32>().unwrap();
        let today = Local::now();

        if day < today.day() && month <= today.month() {
            NaiveDate::from_ymd_opt(today.year() + 1, month, day).unwrap()
        } else {
            NaiveDate::from_ymd_opt(today.year(), month, day).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mitaffald::settings::{Address, AddressId, TraditionalAddress};
    use chrono::{Datelike, Duration, Local};
    use fluent_asserter::{prelude::StrAssertions, *};

    #[tokio::test]
    async fn can_extract_data_using_address_id() {
        let mut remote = mockito::Server::new();
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
                format!("/Adresse/VisAdresseInfo?address-selected-id={}", address_id).as_str(),
            )
            .with_status(200)
            .with_body_from_file("src/mitaffald/remote_responses/container_information.html")
            .create();

        let actual = get_containers(config).await;
        let expected = cotainers_from_container_information_file();

        remote.assert();
        assert_that!(actual.is_ok()).is_true();
        assert_that!(actual.unwrap().as_slice()).is_equal_to(expected.as_slice());
    }

    #[tokio::test]
    async fn can_extract_data_using_traditional_address() {
        let mut remote = mockito::Server::new();
        let config = AffaldVarmeConfig {
            address: Address::FullySpecified(TraditionalAddress {
                street_name: "Kongevejen".to_string(),
                street_no: "100".to_string(),
                postal_code: "8000".to_string(),
                city: "Aarhus C".to_string(),
            }),
            base_url: Url::parse(remote.url().as_str()).unwrap(),
        };

        let remote = remote
            .mock(
                "GET",
                "/Adresse/VisAdresseInfo?address-search=Kongevejen%2C+8000+Aarhus+C&number-search=100&address-selected-postnr=8000",
            )
            .with_status(200)
            .with_body_from_file("src/mitaffald/remote_responses/container_information.html")
            .create();

        let actual = get_containers(config).await;
        let expected = cotainers_from_container_information_file();

        remote.assert();
        assert_that!(actual.is_ok()).is_true();
        assert_that!(actual.unwrap().as_slice()).is_equal_to(expected.as_slice());
    }

    #[tokio::test]
    async fn using_traditional_address_can_detect_address_not_found() {
        let mut remote = mockito::Server::new();
        let config = AffaldVarmeConfig {
            address: Address::FullySpecified(TraditionalAddress {
                street_name: "Kongevejen".to_string(),
                street_no: "100".to_string(),
                postal_code: "8000".to_string(),
                city: "Aarhus C".to_string(),
            }),
            base_url: Url::parse(&remote.url()).unwrap(),
        };

        let remote = remote
            .mock(
                "GET",
                "/Adresse/VisAdresseInfo?address-search=Kongevejen%2C+8000+Aarhus+C&number-search=100&address-selected-postnr=8000",
            )
            .with_status(200)
            .with_body_from_file("src/mitaffald/remote_responses/traditionaladdress_not_found.html")
            .create();

        let actual = get_containers(config).await;

        remote.assert();
        assert_that!(actual.is_err()).is_true();
        assert_that!(actual.unwrap_err()).is_equal_to(": fejl ved opslag på adressen. Kontakt venligst KundeService Affald på mail: kundeservicegenbrug@kredslob.dk eller telefonnummer 77 88 10 10.".to_string());
    }

    #[tokio::test]
    async fn using_addressid_can_detect_address_not_found() {
        let mut remote = mockito::Server::new();
        let address_id = "123".to_string();
        let config = AffaldVarmeConfig {
            address: Address::Id(AddressId {
                id: address_id.clone(),
            }),
            base_url: Url::parse(&remote.url()).unwrap(),
        };

        println!("current dir: {:?}", std::env::current_dir());

        let remote = remote
            .mock(
                "GET",
                format!("/Adresse/VisAdresseInfo?address-selected-id={}", address_id).as_str(),
            )
            .with_status(200)
            .with_body_from_file("src/mitaffald/remote_responses/addressid_not_found.html")
            .create();

        let actual = get_containers(config).await;

        remote.assert();
        assert_that!(actual.is_err()).is_true();
        assert_that!(actual.unwrap_err()).is_equal_to("Søgningen gav intet resultat".to_string());
    }

    #[tokio::test]
    async fn can_handle_server_error() {
        let mut remote = mockito::Server::new();
        let config = AffaldVarmeConfig {
            address: Address::Id(AddressId { id: "123".into() }),
            base_url: Url::parse(remote.url().as_str()).unwrap(),
        };

        let remote = remote
            .mock("GET", mockito::Matcher::Regex(".*".to_string()))
            .with_status(500)
            .create();

        let actual = get_containers(config).await;

        remote.assert();
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

    #[test]
    fn can_calculate_next_date_future() {
        let date_in_the_future = Local::now().date_naive() + Duration::days(1);
        let input = build_container(date_in_the_future);

        let actual = input.get_next_empty();

        assert_that!(actual).is_equal_to(date_in_the_future);
    }

    #[test]
    fn can_calculate_next_date_today() {
        let today = Local::now().date_naive();
        let input = build_container(today);

        let actual = input.get_next_empty();

        assert_that!(actual).is_equal_to(today);
    }

    #[test]
    fn can_calculate_next_date_at_year_end() {
        let today = Local::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);

        let input = build_container(yesterday);

        let actual = input.get_next_empty();
        let expected =
            NaiveDate::from_ymd_opt(yesterday.year() + 1, yesterday.month(), yesterday.day())
                .unwrap();

        assert_that!(actual).is_equal_to(expected);
    }

    fn cotainers_from_container_information_file() -> [Container; 2] {
        [
            Container {
                id: "11064295".to_owned(),
                name: "Restaffald".to_owned(),
                frequency: "1 gang på 2 uger".to_owned(),
                next_empty: "04/08".to_owned(),
                size: "240 L".to_owned(),
            },
            Container {
                id: "12019493".to_owned(),
                name: "Genanvendeligt affald (Glas plast metal og papir pap)".to_owned(),
                frequency: "1 gang på 4 uger".to_owned(),
                next_empty: "03/08".to_owned(),
                size: "240 L".to_owned(),
            },
        ]
    }

    fn build_container(next_empty: NaiveDate) -> Container {
        Container {
            id: "11064295".to_owned(),
            name: "Restaffald".to_owned(),
            frequency: "1 gang på 2 uger".to_owned(),
            next_empty: next_empty.format("%d/%m").to_string(),
            size: "240 L".to_owned(),
        }
    }
}
