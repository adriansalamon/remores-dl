use core::fmt;
use std::hash::Hash;

use chrono::{DateTime, NaiveDateTime, Utc};
use scraper::{Html, Selector};

const REMORES_URL: &str = "https://www.csc.kth.se/cgi-bin/bokning/remores1.4/server/decoder";

pub struct Remores {
    client: reqwest::Client,
    repository: String,
}

type KTHId = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Email {
    KTHEmail(KTHId),
    OtherEmail(String),
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Booking {
    pub time: DateTime<Utc>,
    pub name: String,
    pub email: Email,
}

impl Remores {
    pub fn new(repository: String) -> Self {
        let client = reqwest::Client::new();
        Remores { client, repository }
    }

    pub async fn get_bookings_for(&self, kth_id: String) -> Result<Vec<Booking>, anyhow::Error> {
        let overview = self
            .client
            .get(REMORES_URL)
            .query(&[
                ("request:overview", "yes"),
                ("repository", &self.repository),
                ("shownameemail", "yes"),
            ])
            .send()
            .await?
            .text()
            .await?;

        let document = Html::parse_fragment(&overview);
        let selector = Selector::parse("input").unwrap();

        let sub_lists: Vec<&str> = document
            .select(&selector)
            .filter_map(|el| el.attr("value"))
            .filter(|value| value.ends_with(&kth_id))
            .collect();

        let mut bookings = vec![];
        for sub_list in sub_lists {
            bookings.extend(self.get_sublist(sub_list).await?);
        }

        Ok(bookings)
    }

    async fn get_sublist(&self, sub_list: &str) -> Result<Vec<Booking>, anyhow::Error> {
        let mut bookings = vec![];

        let params = [
            ("event", sub_list),
            ("request:reservation-view", "+Hämta+bokningslista+"),
            ("shownameemail", "yes"),
            ("repository", &self.repository),
        ];
        let content = self
            .client
            .post(REMORES_URL)
            .form(&params)
            .send()
            .await?
            .text()
            .await?;

        let document = Html::parse_fragment(&content);
        let date_selector = Selector::parse("br").unwrap();

        let date = document
            .select(&date_selector)
            .next()
            .and_then(|e| e.next_sibling())
            .and_then(|e| e.next_sibling())
            .and_then(|e| e.first_child())
            .ok_or(anyhow::anyhow!("No date"))?
            .value()
            .as_text()
            .ok_or(anyhow::anyhow!("No date text"))?
            .trim();

        let selector = Selector::parse("input[name=reservation]").unwrap();

        for el in document.select(&selector) {
            let time: &str = el
                .prev_sibling()
                .and_then(|e| e.prev_sibling())
                .and_then(|e| e.first_child())
                .and_then(|e| e.value().as_text())
                .ok_or(anyhow::anyhow!("No time for input"))?;

            let name_node = el
                .next_sibling()
                .ok_or(anyhow::anyhow!("No name for input"))?;

            let email: &str = name_node
                .next_sibling()
                .and_then(|e| e.first_child())
                .and_then(|e| e.first_child())
                .ok_or(anyhow::anyhow!("No email for input"))?
                .value()
                .as_text()
                .ok_or(anyhow::anyhow!("No email text for input"))?;

            let name: &str = name_node
                .value()
                .as_text()
                .ok_or(anyhow::anyhow!("No text for name element"))?
                .trim_end_matches("(")
                .trim();

            let datetime = format!("{} {}", date, time);
            let time = NaiveDateTime::parse_from_str(&datetime, "%y-%m-%d %H:%M")?;

            let parsed_email = match email.to_string() {
                e if e.ends_with("@kth.se") => Email::KTHEmail(e),
                e => Email::OtherEmail(e),
            };

            let booking = Booking {
                time: DateTime::from_naive_utc_and_offset(time, Utc),
                name: name.to_string(),
                email: parsed_email,
            };

            bookings.push(booking);
        }

        Ok(bookings)
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Email::KTHEmail(email) => write!(f, "{}", email),
            Email::OtherEmail(email) => write!(f, "{}", email),
        }
    }
}

// https://www.csc.kth.se/cgi-bin/bokning/remores1.4/server/decoder?request:overview=yes&repository=adk-mastarprov&shownameemail=yes
// https://www.csc.kth.se/cgi-bin/bokning/remores1.4/server/decoder
