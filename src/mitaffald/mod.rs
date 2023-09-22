use crate::settings::Address;
use chrono::{Datelike, Local, NaiveDate};
use easy_scraper::Pattern;
use std::collections::BTreeMap;

pub fn get_containers(address: Address) -> Result<Vec<Container>, String> {
    let remote = fetch_remote_response(address);

    match remote {
        Ok(response) => match response.text() {
            Ok(text) => Ok(extract_container_data(text)),
            Err(err_reading_text) => Err(format!("{:?}", err_reading_text)),
        },
        Err(err) => Err(format!("{:?}", err)),
    }
}

fn fetch_remote_response(address: Address) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let remote_url = build_remote_url(address);

    reqwest::blocking::get(remote_url)
}

// impl reqwest::IntoUrl IntoUrlSealed for Address {}

fn build_remote_url(address: Address) -> String {
    match address {
        Address::Id(x) => {
            format!(
                "https://mitaffald.affaldvarme.dk/Adresse/VisAdresseInfo?address-selected-id={}",
                x.id
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
    use super::*;
    use chrono::{Datelike, Duration, Local};
    use fluent_asserter::*;

    #[test]
    fn test_can_extract_data() {
        let input = std::fs::read_to_string("src/mitaffald/sample_remote_response.html").unwrap();

        let actual = extract_container_data(input);
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

        assert_that!(actual).is_equal_to(expected);
    }

    #[test]
    fn test_can_calculate_next_date_future() {
        let date_in_the_future = Local::now().date_naive() + Duration::days(1);
        let input = build_container(date_in_the_future);

        let actual = input.get_next_empty();

        assert_that!(actual).is_equal_to(date_in_the_future);
    }

    #[test]
    fn test_can_calculate_next_date_today() {
        let today = Local::now().date_naive();
        let input = build_container(today);

        let actual = input.get_next_empty();

        assert_that!(actual).is_equal_to(today);
    }

    #[test]
    fn test_can_calculate_next_date_at_year_end() {
        let today = Local::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);

        let input = build_container(yesterday);

        let actual = input.get_next_empty();
        let expected =
            NaiveDate::from_ymd_opt(yesterday.year() + 1, yesterday.month(), yesterday.day())
                .unwrap();

        assert_that!(actual).is_equal_to(expected);
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
