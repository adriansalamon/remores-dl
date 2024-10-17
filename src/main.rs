use std::{fs, path::Path};

use clap::{Parser, Subcommand};
use remores_dl::{canvas::Canvas, remores::Remores};

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "List available courses on Canvas where you are either a teacher or a TA.")]
    Courses,
    #[clap(about = "List all available assignments for a specific course on Canvas.")]
    Assignments { course_id: String },
    #[clap(about = "Download submissions from Canvas, matching bookings from REMORES.")]
    Download {
        #[clap(
            default_value = "downloads",
            help = "The folder to download the submissions to"
        )]
        folder: String,
        #[clap(short, long, help = "The REMORES repository name")]
        repo: String,
        #[clap(short, long, help = "Your KTH ID, eg. `asalamon`")]
        kth_id: String,
        #[clap(short, long, help = "The Canvas course ID")]
        course: u32,
        #[clap(short, long, help = "The Canvas assignment ID")]
        assignment: u32,
    },
    #[clap(about = "List all student name with bookings from REMORES.")]
    Bookings {
        #[clap(short, long, help = "The REMORES repository name")]
        repo: String,
        #[clap(short, long, help = "Your KTH ID, eg. `asalamon`")]
        kth_id: String,
    },
}

#[derive(Parser)]
#[command(arg_required_else_help(true))]
struct Cli {
    #[clap(
        long,
        env,
        help = "Can be obtained from https://canvas.kth.se/profile/settings"
    )]
    canvas_api_token: String,
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Courses) => {
            let client = Canvas::new(cli.canvas_api_token);
            println!("Finding courses on Canvas...");

            let courses = client.get_courses().await?;

            println!("Available courses:");
            for course in courses {
                println!("  {}: {}", course.id, course.name);
            }
        }
        Some(Commands::Assignments { course_id }) => {
            let client = Canvas::new(cli.canvas_api_token);
            println!("Finding assignments for course {} on Canvas...", course_id);

            let assignments = client.get_assignments(course_id).await?;

            println!("Available assignments:");
            for assignment in assignments {
                println!("  {}: {}", assignment.id, assignment.name);
            }
        }
        Some(Commands::Download {
            kth_id,
            folder,
            repo,
            course,
            assignment,
        }) => {
            println!("Finding bookings for {} on REMORES...", repo);
            let remores: Remores = Remores::new(repo.to_string());
            let bookings = remores.get_bookings_for(kth_id.to_string()).await?;

            println!("Found {} bookings", bookings.len());

            println!(
                "Finding submissions assignment {} in course {} on Canvas...",
                assignment, course
            );
            let canvas = Canvas::new(cli.canvas_api_token);
            let bookings_with_submissions = canvas
                .get_assignment_submissions(course, assignment, &bookings)
                .await?;

            let n_bookings_with_submissions = bookings_with_submissions
                .iter()
                .filter(|(_, submission)| submission.is_some())
                .count();
            println!(
                "Found matching submissions for {} bookings",
                n_bookings_with_submissions
            );

            for (booking, _) in bookings_with_submissions
                .iter()
                .filter(|(_, submission)| submission.is_none())
            {
                println!(
                    "[Warn]: No submission found for booking: {}, {} @ {}",
                    booking.name, booking.email, booking.time
                );
            }

            println!("Downloading submissions to {}...", folder);

            let folder = Path::new(folder);
            fs::create_dir_all(folder)?;

            for (booking, submission) in bookings_with_submissions {
                if let Some(submission) = submission {
                    let file_name = format!(
                        "{}-{}",
                        booking.time.format("%Y%m%d%H%M"),
                        submission.user.name
                    );
                    match canvas
                        .download_submission(&submission, folder, file_name.as_str())
                        .await
                    {
                        Ok(paths) => {
                            for path in paths {
                                println!("Downloaded submission to {}", path.display());
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to download submission {}: {}", submission.user, e)
                        }
                    }
                }
            }
        }
        Some(Commands::Bookings { repo, kth_id }) => {
            let remores = Remores::new(repo.to_string());

            println!("Finding bookings for {} on REMORES...", repo);
            let bookings = remores.get_bookings_for(kth_id.to_string()).await?;

            let names = bookings
                .iter()
                .map(|booking| format!("\"{}\"", booking.name))
                .collect::<Vec<String>>()
                .join(",");

            println!("Found {} bookings:", bookings.len());
            print!("{}\n", names);
        }
        None => {
            eprintln!("No command provided");
        }
    }

    Ok(())
}
