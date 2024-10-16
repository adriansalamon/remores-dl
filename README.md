## Remores-dl

Simple CLI tool to download canvas submissions for students that have
booked a time slot on REMORES.

It scrapes the REMORES website, and finds time slots that have been
booked by students. It then tries to find matching submissions on
canvas, and downloads them to a specified directory.

You will need a `CANVAS_API_TOKEN` environment variable set to a valid
Canvas API token. You can generate one by going to `Account ->
Settings -> Approved Integrations` on Canvas.

### Usage

```bash
remores-dl help
```

List courses:

```bash
remores-dl list-courses
```

List assignments for a course:

```bash
remores-dl list-assignments <course_id>
```

Download submissions for an assignment, for students that have booked
a time slot with your KTH ID:

```bash
remores-dl download --kth-id <kth_id> --repo <adk-mastarprov> --course <id> --assignment <id>
```

### Building

```bash
cargo build --release
```

The binary will be located at `target/release/remores-dl`, and can be
moved to a directory in your PATH.
