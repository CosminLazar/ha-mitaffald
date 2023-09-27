use crate::{settings::Address, AffaldVarmeConfig};
use chrono::{Datelike, Local, NaiveDate};
use easy_scraper::Pattern;
use std::collections::BTreeMap;

pub fn get_containers(config: AffaldVarmeConfig) -> Result<Vec<Container>, String> {
    let response = fetch_remote_response(config);

    if response.is_err() {
        return Err(format!("Error connecting: {:?}", response.err()));
    }

    let response = response.unwrap();
    if !response.status().is_success() {
        return Err(format!("Unexpected status code: {:?}", response.status()));
    }

    match response.text() {
        Ok(text) => Ok(extract_container_data(text)),
        Err(err_reading_text) => Err(format!(
            "Error reading response content: {:?}",
            err_reading_text
        )),
    }
}

fn fetch_remote_response(
    config: AffaldVarmeConfig,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let remote_url = build_remote_url(config);

    reqwest::blocking::get(remote_url)
}

fn build_remote_url(config: AffaldVarmeConfig) -> String {
    //todo: can we find a nicer way to compose a URL?
    match config.address {
        Address::Id(x) => {
            format!(
                "{}/Adresse/VisAdresseInfo?address-selected-id={}",
                config.base_url, x.id
            )
        }
        Address::FullySpecified(_) => todo!(),
    }
}

fn extract_container_data(html: String) -> Vec<Container> {
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
        .matches(&html)
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
    use crate::AddressId;

    use super::*;
    use chrono::{Datelike, Duration, Local};
    use fluent_asserter::*;

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

    #[test]
    fn can_extract_data() {
        let mut remote = mockito::Server::new();
        let address_id = "123".to_string();
        let config = AffaldVarmeConfig {
            address: Address::Id(AddressId {
                id: address_id.clone(),
            }),
            base_url: remote.url(),
        };

        let remote = remote
            .mock(
                "GET",
                format!("/Adresse/VisAdresseInfo?address-selected-id={}", address_id).as_str(),
            )
            .with_status(200)
            .with_body_from_file("src/mitaffald/sample_remote_response.html")
            .create();

        let actual = get_containers(config);
        let expected = vec![
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
        ];

        remote.assert();

        assert!(matches!(actual, Ok(_)));
        assert_that!(actual.is_err()).is_equal_to(false);

        assert_that!(actual.unwrap()).is_equal_to(expected);
    }

    #[test]
    fn can_handle_error_responses() {
        let mut remote = mockito::Server::new();
        let config = AffaldVarmeConfig {
            address: Address::Id(AddressId { id: "123".into() }),
            base_url: remote.url(),
        };

        let remote = remote
            .mock("GET", mockito::Matcher::Regex(".*".to_string()))
            .with_status(500)
            .create();

        let actual = get_containers(config);

        remote.assert();
        assert!(matches!(actual, Err(msg) if msg.contains("Unexpected status code")));
    }

    #[test]
    fn can_handle_no_responses() {
        let config = AffaldVarmeConfig {
            address: Address::Id(AddressId { id: "123".into() }),
            base_url: "http://127.0.0.1:123123".to_string(),
        };

        let actual = get_containers(config);

        assert!(matches!(actual, Err(x) if x.contains("Error connecting")));
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
