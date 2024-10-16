use anyhow::Ok;
use chrono::{DateTime, Utc};
use core::fmt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use std::{fs::File, io::Write};

use crate::remores::{Booking, Email};

const API_URL: &str = "https://canvas.kth.se/api/v1";

pub struct Canvas {
    client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
struct Enrollment {
    r#type: String,
}

#[derive(Deserialize, Debug)]
pub struct Course {
    pub name: String,
    pub id: u64,
    created_at: Option<DateTime<Utc>>,
    enrollments: Vec<Enrollment>,
}

#[derive(Deserialize, Debug)]
pub struct Assignment {
    pub id: u64,
    pub name: String,
    due_at: Option<DateTime<Utc>>,
    published: bool,
    grading_type: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Submission {
    pub id: u64,
    attachments: Option<Vec<Attachment>>,
    pub user: User,
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub name: String,
    #[serde(rename = "login_id")]
    email: String,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.email)
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Attachment {
    url: String,
    display_name: String,
}

const GRADE_KEYS: [&str; 3] = ["pass_fail", "points", "letter_grade"];

impl Canvas {
    pub fn new(api_token: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_token)).unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Canvas { client }
    }

    pub async fn get_courses(&self) -> Result<Vec<Course>, anyhow::Error> {
        let mut courses: Vec<Course> = self
            .client
            .get(&format!("{}/courses", API_URL))
            .query(&[("per_page", 100)])
            .send()
            .await?
            .json()
            .await?;

        courses = courses
            .into_iter()
            .filter(|course: &Course| {
                course
                    .enrollments
                    .iter()
                    .any(|enrollment| enrollment.r#type != "student")
            })
            .collect();

        courses.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(courses)
    }

    pub async fn get_assignments(&self, course_id: &str) -> Result<Vec<Assignment>, anyhow::Error> {
        let mut assignments: Vec<Assignment> = self
            .client
            .get(&format!("{}/courses/{}/assignments", API_URL, course_id))
            .query(&[("per_page", 100)])
            .send()
            .await?
            .json()
            .await?;

        assignments = assignments
            .into_iter()
            .filter(|a| a.published)
            .filter(|a| GRADE_KEYS.contains(&a.grading_type.as_str()))
            .collect();

        assignments.sort_by(|a, b| {
            a.due_at
                .unwrap_or_else(|| Utc::now())
                .cmp(&b.due_at.unwrap_or_else(|| Utc::now()))
        });

        Ok(assignments)
    }

    pub async fn get_assignment_submissions(
        &self,
        course: &u32,
        assignment: &u32,
        bookings: &[crate::remores::Booking],
    ) -> Result<HashMap<Booking, Option<Submission>>, anyhow::Error> {
        let mut submissions: Vec<Submission> = self
            .get_paginated_data(&format!(
                "{}/courses/{}/assignments/{}/submissions?include[]=user",
                API_URL, course, assignment
            ))
            .await?;

        let mut booking_map: HashMap<Booking, Option<Submission>> = bookings
            .iter()
            .map(|booking| (booking.clone(), None))
            .collect();

        for booking in bookings {
            // Check if the booking kth email is in the submissions
            if let Some(submission) = submissions
                .iter()
                .find(|submission| Email::KTHEmail(submission.user.email.clone()) == booking.email)
            {
                booking_map.insert(booking.clone(), Some(submission.clone()));
            } else {
                // If not, try to find a submission with a similar name,
                // not perfect but better than nothing
                submissions.sort_by(|a, b| {
                    let a_sim = strsim::jaro(&a.user.name, &booking.name);
                    let b_sim = strsim::jaro(&b.user.name, &booking.name);
                    a_sim.partial_cmp(&b_sim).unwrap()
                });

                if let Some(submission) = submissions.pop() {
                    if strsim::jaro(&submission.user.name, &booking.name) > 0.8 {
                        booking_map.insert(booking.clone(), Some(submission));
                    }
                }
            }
        }

        Ok(booking_map)
    }

    pub async fn download_submission<T: AsRef<Path>>(
        &self,
        submission: &Submission,
        folder: T,
        file_name: &str,
    ) -> Result<Vec<PathBuf>, anyhow::Error> {
        if submission.attachments.is_none() {
            anyhow::bail!("No attachments found for submission");
        }

        let mut paths = vec![];
        for attachment in submission.attachments.as_ref().unwrap_or(&vec![]) {
            let file_name = format!("{}-{}", file_name, attachment.display_name);
            let path = PathBuf::from(folder.as_ref()).join(file_name);
            paths.push(path.clone());
            println!("Downloading attachment to {}", path.display());

            let mut file = File::create(path)?;
            let resp = self.client.get(&attachment.url).send().await?;
            let bytes = resp.bytes().await?;

            file.write_all(&bytes)?;
        }

        Ok(paths)
    }

    async fn get_paginated_data<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> Result<Vec<T>, anyhow::Error> {
        let mut data = vec![];

        let mut url = url.to_string();

        loop {
            let resp = self
                .client
                .get(&url)
                .query(&[("per_page", 100)])
                .send()
                .await?;
            let headers = resp.headers().clone();

            data.extend(resp.json::<Vec<T>>().await?);

            if let Some(link) = headers.get("link") {
                let link = link
                    .to_str()?
                    .split(",")
                    .find(|link| link.ends_with("rel=\"next\""))
                    .map(|link| {
                        link.trim_start_matches("<")
                            .split_once(">")
                            .map(|(link, _)| link)
                    })
                    .flatten();

                if let Some(link) = link {
                    url = link.to_string();
                } else {
                    break;
                }
            }
        }

        Ok(data)
    }
}
